//! 廉价的「画面上有没有强化卡片」检查：只读约 100 个像素点，耗时几微秒。
//! 对局中每次心跳先跑这个检查，看到卡片才跑 OCR（约 100–200 ms）。
//!
//! 卡片版式与判定方法改编自 Hexsight（https://github.com/bard58/hexsight，src/ocr.rs 的
//! `cards_visible`），按 MIT 许可使用（Hexsight 为 MIT / Apache-2.0 双许可），见 THIRD-PARTY-NOTICES.md。
//! 版式以 1080p 游戏画面为基准：三张卡水平居中，间距 368；卡框约在画面高度 30%–60%；
//! 卡身是很暗的青色（约 r 1, g 15, b 16）。

/// 相邻两张卡中心的间距（1080p 下的像素，随游戏高度缩放）
const CARD_PITCH: f32 = 368.0;
/// 卡片中心到亮色卡框中线、以及到卡框内侧暗色卡身的水平距离
const CARD_RIM: f32 = 149.0;
const CARD_INNER: f32 = 118.0;
/// 穿过卡框的行，以及描述文字下方空白卡身的行（游戏高度的比例）
const RIM_ROWS: [f32; 6] = [0.30, 0.36, 0.42, 0.48, 0.54, 0.60];
const BODY_ROWS: [f32; 2] = [0.585, 0.600];
const BODY_DX: [f32; 7] = [-90.0, -60.0, -30.0, 0.0, 30.0, 60.0, 90.0];
/// 14 个卡身点里有这么多是卡身颜色，或 12 对卡框点里有这么多比内侧亮，就算这张卡在
const BODY_HITS: usize = 8;
const RIM_HITS: usize = 7;
/// 两张就够：提示框或翻牌动画可能挡住第三张
const CARDS_NEEDED: usize = 2;

/// 卡片上沿大约在游戏高度的这个比例处（下沿约 0.645，其下是重随按钮）
pub const CARD_TOP: f32 = 0.265;

/// 卡身颜色：很暗的青色；画面淡入时是它的变暗版本
fn is_card_body(r: i32, g: i32, b: i32) -> bool {
    r <= 12 && (7..=32).contains(&g) && (7..=36).contains(&b) && (g - b).abs() <= 7 && g > r + 4
}

/// 截图（BGRA，宽 `w` 高 `h`）在游戏画面中的位置与游戏画面尺寸
pub struct Frame<'a> {
    pub bgra: &'a [u8],
    pub w: usize,
    pub h: usize,
    /// 截图左上角在游戏画面中的坐标
    pub left: i32,
    pub top: i32,
    pub game_w: i32,
    pub game_h: i32,
}

/// 三张卡里至少两张可见时返回 true。截图不完整时返回 true（不做判断，交给 OCR）。
pub fn cards_visible(f: &Frame) -> bool {
    if f.w < 64 || f.h < 64 || f.bgra.len() < f.w * f.h * 4 || f.game_h <= 0 {
        return true;
    }
    let unit = f.game_h as f32 / 1080.0;
    let cx = f.game_w as f32 / 2.0;
    let rgb = |card: f32, dx: f32, row: f32| -> Option<[i32; 3]> {
        let x = (cx + (card * CARD_PITCH + dx) * unit).round() as i32 - f.left;
        let y = (row * f.game_h as f32).round() as i32 - f.top;
        if x < 0 || y < 0 || x as usize >= f.w || y as usize >= f.h {
            return None;
        }
        let i = (y as usize * f.w + x as usize) * 4;
        Some([f.bgra[i + 2] as i32, f.bgra[i + 1] as i32, f.bgra[i] as i32])
    };
    let lum = |p: [i32; 3]| (p[0] * 77 + p[1] * 150 + p[2] * 29) >> 8;
    let mut cards = 0;
    for card in [-1.0, 0.0, 1.0] {
        let body = BODY_ROWS
            .iter()
            .flat_map(|&row| BODY_DX.iter().map(move |&dx| (dx, row)))
            .filter(|&(dx, row)| rgb(card, dx, row).is_some_and(|p| is_card_body(p[0], p[1], p[2])))
            .count();
        let rim = RIM_ROWS
            .iter()
            .flat_map(|&row| [-1.0, 1.0].map(move |side| (side, row)))
            .filter(|&(side, row)| match (rgb(card, side * CARD_RIM, row), rgb(card, side * CARD_INNER, row)) {
                (Some(rim), Some(inner)) => lum(rim) - lum(inner) >= 25,
                _ => false,
            })
            .count();
        if body >= BODY_HITS || rim >= RIM_HITS {
            cards += 1;
        }
    }
    cards >= CARDS_NEEDED
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 画一张 1920x1080 的游戏画面，按需在指定位置画卡片，截取与 win.rs 相同的区域
    fn frame(cards: &[f32]) -> (Vec<u8>, usize, usize, i32, i32) {
        let (gw, gh) = (1920i32, 1080i32);
        let (left, top) = ((gw as f32 * 0.10) as i32, (gh as f32 * 0.10) as i32);
        let (w, h) = ((gw as f32 * 0.80) as usize, (gh as f32 * 0.68) as usize);
        // 背景：游戏场景的随机暗绿色
        let mut px = vec![0u8; w * h * 4];
        for (i, p) in px.chunks_exact_mut(4).enumerate() {
            let v = (i * 7919 % 60) as u8;
            p.copy_from_slice(&[20 + v / 2, 60 + v, 40 + v / 3, 255]);
        }
        for &card in cards {
            let cx = gw as f32 / 2.0 + card * CARD_PITCH;
            for y in (0.27 * gh as f32) as i32..(0.63 * gh as f32) as i32 {
                for x in (cx - 160.0) as i32..(cx + 160.0) as i32 {
                    let (sx, sy) = (x - left, y - top);
                    if sx < 0 || sy < 0 || sx as usize >= w || sy as usize >= h {
                        continue;
                    }
                    let d = (x as f32 - cx).abs();
                    // 卡框（亮金色）与卡身（暗青色）
                    let c: [u8; 3] = if (140.0..=158.0).contains(&d) { [200, 170, 90] } else { [1, 15, 16] };
                    let i = (sy as usize * w + sx as usize) * 4;
                    px[i..i + 4].copy_from_slice(&[c[2], c[1], c[0], 255]);
                }
            }
        }
        (px, w, h, left, top)
    }

    fn check(cards: &[f32]) -> bool {
        let (px, w, h, left, top) = frame(cards);
        cards_visible(&Frame { bgra: &px, w, h, left, top, game_w: 1920, game_h: 1080 })
    }

    #[test]
    fn sees_three_or_two_cards() {
        assert!(check(&[-1.0, 0.0, 1.0]));
        assert!(check(&[-1.0, 1.0]));
    }

    #[test]
    fn ignores_game_scene_and_single_card() {
        assert!(!check(&[]));
        assert!(!check(&[0.0]));
    }
}
