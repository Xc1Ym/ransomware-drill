//! 其它平台的占位实现。
//!
//! 套件目前只支持 macOS 与 Windows；在不支持的平台上，
//! 文件锁定/恢复仍然可用，只是无法更换桌面壁纸。

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

pub fn set_wallpaper(_path: &Path) -> Result<()> {
    bail!("当前平台暂不支持自动更换桌面壁纸，请手动设置。")
}

pub fn get_wallpaper() -> Result<Option<PathBuf>> {
    Ok(None)
}
