//! 发现 LCU（英雄联盟客户端）的端口与令牌。
//!
//! 合规说明：这里只做两件事——
//! 1. 读取客户端安装目录下由客户端自己写出的 `lockfile`；
//! 2. （仅 Windows 兜底）通过系统 WMI 查询 `LeagueClientUx.exe` 的启动参数。
//!
//! 不打开游戏进程（League of Legends.exe），不读写任何进程内存，不注入。

use std::path::{Path, PathBuf};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credentials {
    pub port: u16,
    pub password: String,
}

/// lockfile 格式：`LeagueClient:<pid>:<port>:<password>:https`
pub fn parse_lockfile(s: &str) -> Option<Credentials> {
    let parts: Vec<&str> = s.trim().split(':').collect();
    if parts.len() < 5 {
        return None;
    }
    Some(Credentials {
        port: parts[2].parse().ok()?,
        password: parts[3].to_string(),
    })
}

/// 从启动参数中提取 `--app-port=` 与 `--remoting-auth-token=`。
#[cfg_attr(not(windows), allow(dead_code))]
pub fn parse_cmdline(s: &str) -> Option<Credentials> {
    fn arg<'a>(s: &'a str, key: &str) -> Option<&'a str> {
        let start = s.find(key)? + key.len();
        let rest = &s[start..];
        let end = rest
            .find(|c: char| c == '"' || c.is_whitespace())
            .unwrap_or(rest.len());
        Some(&rest[..end])
    }
    Some(Credentials {
        port: arg(s, "--app-port=")?.parse().ok()?,
        password: arg(s, "--remoting-auth-token=")?.to_string(),
    })
}

fn read_lockfile(dir: &Path) -> Option<Credentials> {
    std::fs::read_to_string(dir.join("lockfile"))
        .ok()
        .and_then(|s| parse_lockfile(&s))
}

const CLIENT_NAMES: [&str; 2] = ["LeagueClientUx.exe", "LeagueClient.exe"];

pub struct Discoverer {
    sys: System,
    /// WMI 兜底结果按 pid 缓存，避免反复拉起 powershell。
    wmi_cache: Option<(Pid, Credentials)>,
}

impl Discoverer {
    pub fn new() -> Self {
        Self { sys: System::new(), wmi_cache: None }
    }

    pub fn discover(&mut self, custom_dir: Option<&str>) -> Option<Credentials> {
        if let Some(dir) = custom_dir.filter(|d| !d.trim().is_empty()) {
            if let Some(c) = read_lockfile(Path::new(dir.trim())) {
                return Some(c);
            }
        }

        // 只刷新进程名（成本很低），找到客户端后再单独查询其可执行文件路径。
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing(),
        );
        let pid = self
            .sys
            .processes()
            .iter()
            .find(|(_, p)| CLIENT_NAMES.iter().any(|n| p.name() == *n))
            .map(|(pid, _)| *pid)?;

        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            false,
            ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
        );
        if let Some(exe) = self.sys.process(pid).and_then(|p| p.exe()) {
            let dirs: Vec<PathBuf> = exe.ancestors().skip(1).take(3).map(Path::to_path_buf).collect();
            for d in dirs {
                if let Some(c) = read_lockfile(&d) {
                    return Some(c);
                }
            }
        }

        if let Some((cached_pid, c)) = &self.wmi_cache {
            if *cached_pid == pid {
                return Some(c.clone());
            }
        }
        let c = cmdline_via_wmi()?;
        self.wmi_cache = Some((pid, c.clone()));
        Some(c)
    }
}

#[cfg(windows)]
fn cmdline_via_wmi() -> Option<Credentials> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "(Get-CimInstance Win32_Process -Filter \"name='LeagueClientUx.exe'\").CommandLine",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    parse_cmdline(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(not(windows))]
fn cmdline_via_wmi() -> Option<Credentials> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockfile() {
        let c = parse_lockfile("LeagueClient:12345:54321:abcDEF_123:https").unwrap();
        assert_eq!(c.port, 54321);
        assert_eq!(c.password, "abcDEF_123");
        assert!(parse_lockfile("garbage").is_none());
    }

    #[test]
    fn cmdline() {
        let s = r#""C:/Riot Games/League of Legends/LeagueClientUx.exe" "--riotclient-auth-token=x" "--app-port=61234" "--remoting-auth-token=tok-EN_1" "--install-directory=D:\英雄联盟""#;
        let c = parse_cmdline(s).unwrap();
        assert_eq!(c.port, 61234);
        assert_eq!(c.password, "tok-EN_1");
        let s2 = "LeagueClientUx.exe --app-port=1000 --remoting-auth-token=abc --foo";
        assert_eq!(parse_cmdline(s2).unwrap().password, "abc");
    }
}
