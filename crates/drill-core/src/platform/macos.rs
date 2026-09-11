//! macOS 壁纸读写，通过 `osascript` 驱动 System Events。
//!
//! 首次运行时系统会弹出「允许控制『系统事件』」的授权请求，必须点「好」，
//! 否则设置壁纸会失败。这一点已写进 README 的演练准备清单。

use super::applescript_escape;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

fn run_osascript(script: &str) -> Result<String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .context("调用 osascript 失败，请确认系统为 macOS 且 osascript 可用")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "osascript 执行失败：{}\n\
             若提示「不允许发送 Apple 事件」，请在\n\
             『系统设置 → 隐私与安全性 → 自动化』中允许本程序控制『系统事件』。",
            stderr.trim()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn set_wallpaper(path: &Path) -> Result<()> {
    if !path.exists() {
        bail!("壁纸文件不存在：{}", path.display());
    }
    let escaped = applescript_escape(path);
    let script = format!(
        "tell application \"System Events\" to tell every desktop to set picture to \"{escaped}\""
    );
    run_osascript(&script)?;
    Ok(())
}

/// 读取当前所有桌面的壁纸，返回第一个（多显示器时各桌面可能不同）。
pub fn get_wallpaper() -> Result<Option<PathBuf>> {
    let script = "tell application \"System Events\" to get picture of desktop 1";
    let out = run_osascript(script)?;
    if out.is_empty() {
        return Ok(None);
    }
    Ok(Some(PathBuf::from(out)))
}
