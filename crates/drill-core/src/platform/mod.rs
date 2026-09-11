//! 平台适配层：桌面壁纸读写。
//!
//! 之所以要把壁纸操作单独抽出来，是因为它是整套工具里**唯一会改变系统状态**
//! 的动作（其它动作只动文件名字）。因此它必须同时提供「读取」能力，
//! 以便在锁定前备份原壁纸、恢复时精确还原。

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{get_wallpaper, set_wallpaper};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{get_wallpaper, set_wallpaper};

#[cfg(not(any(target_os = "macos", windows)))]
mod unsupported;
#[cfg(not(any(target_os = "macos", windows)))]
pub use unsupported::{get_wallpaper, set_wallpaper};

/// 把 `path` 转成可以安全嵌进 AppleScript 字符串字面量的形式。
#[cfg(target_os = "macos")]
pub(crate) fn applescript_escape(path: &std::path::Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn 反斜杠与引号会被转义() {
        let p = std::path::PathBuf::from(r#"/tmp/a"b\c.png"#);
        let escaped = applescript_escape(&p);
        assert_eq!(escaped, r#"/tmp/a\"b\\c.png"#);
    }
}
