//! Windows 实现：GDI 截屏 + Windows.Media.Ocr（系统自带，无需额外模型）。

use super::{EngineUnavailable, Line, Outcome, Scan};
use std::collections::HashMap;
use anyhow::{anyhow, bail, Result};
use std::ffi::c_void;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, ClientToScreen, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
    GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
};
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetClientRect, GetForegroundWindow};

/// 截取区域：强化卡片位于画面中部，只截这一块以降低 OCR 耗时
const REGION: (f32, f32, f32, f32) = (0.10, 0.10, 0.90, 0.78); // left, top, right, bottom

type Reply = tokio::sync::oneshot::Sender<Result<Outcome>>;

enum Msg {
    /// (语言, 是否先做廉价检查：看不到卡片就不跑 OCR)
    Scan(String, bool, Reply),
    Installed(tokio::sync::oneshot::Sender<Vec<String>>),
}

/// WinRT 需要初始化过的线程，所以 OCR 固定在一个专用线程里执行；
/// 每种语言的引擎首次使用时创建并缓存。
pub struct Worker {
    tx: std::sync::mpsc::Sender<Msg>,
}

impl Worker {
    pub fn spawn() -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<Msg>();
        std::thread::Builder::new()
            .name("ocr".into())
            .spawn(move || {
                let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
                let mut engines: HashMap<String, Option<OcrEngine>> = HashMap::new();
                for msg in rx {
                    match msg {
                        Msg::Scan(lang, probe, reply) => {
                            let engine = engines.entry(lang.clone()).or_insert_with(|| create_engine(&lang));
                            let r = match engine {
                                Some(e) => scan(e, probe),
                                None => Err(EngineUnavailable(lang).into()),
                            };
                            let _ = reply.send(r);
                        }
                        Msg::Installed(reply) => {
                            let _ = reply.send(installed_langs());
                        }
                    }
                }
            })
            .expect("无法创建 OCR 线程");
        Self { tx }
    }

    pub async fn scan(&self, lang: &str, probe: bool) -> Result<Outcome> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx.send(Msg::Scan(lang.to_string(), probe, tx)).map_err(|_| anyhow!("OCR worker exited"))?;
        rx.await.map_err(|_| anyhow!("OCR worker not responding"))?
    }

    /// 系统已安装的 OCR 语言标签，如 ["zh-Hans-CN", "en-US"]
    pub async fn installed(&self) -> Vec<String> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        if self.tx.send(Msg::Installed(tx)).is_err() {
            return Vec::new();
        }
        rx.await.unwrap_or_default()
    }
}

fn installed_langs() -> Vec<String> {
    let Ok(list) = OcrEngine::AvailableRecognizerLanguages() else { return Vec::new() };
    list.into_iter().filter_map(|l| l.LanguageTag().ok().map(|t| t.to_string())).collect()
}

/// 只接受与请求语言一致的引擎；不回退到用户界面语言，避免用英文引擎去读中文卡片
fn create_engine(lang: &str) -> Option<OcrEngine> {
    let l = Language::CreateLanguage(&HSTRING::from(lang)).ok()?;
    if !OcrEngine::IsLanguageSupported(&l).unwrap_or(false) {
        return None;
    }
    OcrEngine::TryCreateFromLanguage(&l).ok()
}

struct Capture {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    bgra: Vec<u8>,
}

fn capture(left: i32, top: i32, width: i32, height: i32) -> Result<Capture> {
    unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            bail!("GetDC failed");
        }
        let mem = CreateCompatibleDC(Some(screen));
        let bmp = CreateCompatibleBitmap(screen, width, height);
        let old = SelectObject(mem, bmp.into());
        let blt = BitBlt(mem, 0, 0, width, height, Some(screen), left, top, SRCCOPY);

        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // 负数 = 自上而下的行序
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bgra = vec![0u8; (width * height * 4) as usize];
        let rows = GetDIBits(
            mem,
            bmp,
            0,
            height as u32,
            Some(bgra.as_mut_ptr() as *mut c_void),
            &mut info,
            DIB_RGB_COLORS,
        );

        SelectObject(mem, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);

        blt?;
        if rows == 0 {
            bail!("GetDIBits failed");
        }
        for px in bgra.chunks_exact_mut(4) {
            px[3] = 255;
        }
        Ok(Capture { left, top, width, height, bgra })
    }
}

/// 只按窗口类名查找游戏窗口，不打开游戏进程
fn find_game() -> Option<windows::Win32::Foundation::HWND> {
    let hwnd = unsafe { FindWindowW(w!("RiotWindowClass"), PCWSTR::null()) }.ok()?;
    (!hwnd.is_invalid()).then_some(hwnd)
}

/// 游戏画面在屏幕上的区域（x, y, w, h），找不到或太小返回 None
pub fn game_rect() -> Option<(i32, i32, i32, i32)> {
    let hwnd = find_game()?;
    let mut rc = RECT::default();
    unsafe { GetClientRect(hwnd, &mut rc) }.ok()?;
    let mut origin = POINT { x: 0, y: 0 };
    if !unsafe { ClientToScreen(hwnd, &mut origin) }.as_bool() {
        return None;
    }
    let (cw, ch) = (rc.right - rc.left, rc.bottom - rc.top);
    (cw >= 320 && ch >= 240).then_some((origin.x, origin.y, cw, ch))
}

fn scan(engine: &OcrEngine, probe: bool) -> Result<Outcome> {
    let Some(hwnd) = find_game() else { return Ok(Outcome::NoGame) };
    if unsafe { GetForegroundWindow() } != hwnd {
        return Ok(Outcome::Background);
    }
    let Some((ox0, oy0, cw, ch)) = game_rect() else { return Ok(Outcome::NoGame) };
    let origin = POINT { x: ox0, y: oy0 };

    let (l, t, r, b) = REGION;
    let cap = capture(
        origin.x + (cw as f32 * l) as i32,
        origin.y + (ch as f32 * t) as i32,
        (cw as f32 * (r - l)) as i32,
        (ch as f32 * (b - t)) as i32,
    )?;

    // 心跳扫描：先用几微秒的像素检查看有没有卡片，没有就不跑 OCR
    if probe {
        let frame = super::cards::Frame {
            bgra: &cap.bgra,
            w: cap.width as usize,
            h: cap.height as usize,
            left: cap.left - origin.x,
            top: cap.top - origin.y,
            game_w: cw,
            game_h: ch,
        };
        if !super::cards::cards_visible(&frame) {
            return Ok(Outcome::NoCards);
        }
    }

    let writer = DataWriter::new()?;
    writer.WriteBytes(&cap.bgra)?;
    let buffer = writer.DetachBuffer()?;
    let bitmap = SoftwareBitmap::CreateCopyFromBuffer(&buffer, BitmapPixelFormat::Bgra8, cap.width, cap.height)?;
    let result = engine.RecognizeAsync(&bitmap)?.join()?;

    let (ox, oy) = ((cap.left - origin.x) as f32, (cap.top - origin.y) as f32);
    let mut lines = Vec::new();
    for line in result.Lines()? {
        let text = line.Text()?.to_string();
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for word in line.Words()? {
            let r = word.BoundingRect()?;
            x0 = x0.min(r.X);
            y0 = y0.min(r.Y);
            x1 = x1.max(r.X + r.Width);
            y1 = y1.max(r.Y + r.Height);
        }
        if x0 > x1 {
            continue;
        }
        lines.push(Line {
            text,
            x: (ox + x0) / cw as f32,
            y: (oy + y0) / ch as f32,
            w: (x1 - x0) / cw as f32,
            h: (y1 - y0) / ch as f32,
        });
    }
    Ok(Outcome::Done(Scan { rect: (origin.x, origin.y, cw, ch), lines }))
}
