//! Windows 壁纸读写，通过 `user32.dll` 的 `SystemParametersInfoW`。
//!
//! 读写都用同一个 API（`SPI_SETDESKWALLPAPER` / `SPI_GETDESKWALLPAPER`），
//! 避免直接操作注册表。写入时带 `SPIF_UPDATEINIFILE`，保证登录后仍然生效；
//! 恢复工具会用 `SPI_GETDESKWALLPAPER` 读回原路径再设回去。

use anyhow::{bail, Result};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SystemParametersInfoW, SPI_GETDESKWALLPAPER, SPI_SETDESKWALLPAPER, SPIF_SENDCHANGE,
    SPIF_UPDATEINIFILE,
};

/// Win32 路径缓冲区长度上限。
const MAX_PATH_W: usize = 260;

fn to_wide_nul(p: &Path) -> Vec<u16> {
    p.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

pub fn set_wallpaper(path: &Path) -> Result<()> {
    if !path.exists() {
        bail!("壁纸文件不存在：{}", path.display());
    }

    let wide = to_wide_nul(path);
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            wide.as_ptr() as *mut core::ffi::c_void,
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        )
    };

    if ok == 0 {
        bail!(
            "设置桌面壁纸失败：{}",
            std::io::Error::last_os_error()
        );
    }
    Ok(())
}

pub fn get_wallpaper() -> Result<Option<PathBuf>> {
    let mut buf = [0u16; MAX_PATH_W];
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETDESKWALLPAPER,
            MAX_PATH_W as u32,
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            0,
        )
    };

    if ok == 0 {
        return Ok(None);
    }

    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    if len == 0 {
        return Ok(None);
    }
    Ok(Some(PathBuf::from(String::from_utf16_lossy(&buf[..len]))))
}
