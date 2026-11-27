//! 便携版：所有数据（设置、缓存、WebView2 用户数据）放在 exe 同级的 `haidou-data/` 目录，
//! 拷走整个文件夹即可。exe 所在目录不可写（如放在 Program Files）时回退到系统 AppData。

use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};

fn portable_dir() -> Option<PathBuf> {
    let dir = std::env::current_exe().ok()?.parent()?.join("haidou-data");
    std::fs::create_dir_all(&dir).ok()?;
    let probe = dir.join(".write-test");
    std::fs::write(&probe, b"ok").ok()?;
    let _ = std::fs::remove_file(probe);
    Some(dir)
}

pub fn data_dir(app: &AppHandle) -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        // 开发模式下 exe 在 target/ 里，不污染源码目录之外的地方也无妨；统一走便携逻辑
        portable_dir()
            .or_else(|| app.path().app_data_dir().ok())
            .unwrap_or_else(|| PathBuf::from("haidou-data"))
    })
    .clone()
}

pub fn cache_dir(app: &AppHandle) -> PathBuf {
    data_dir(app).join("cache")
}
