//! 安全护栏。
//!
//! 演练工具最大的风险是「演示时不小心锁了真实目录」。本模块在动手之前
//! 做两层检查：
//!
//! 1. **根目录黑名单**——命中系统目录或用户关键目录直接中止；
//! 2. **规模上限**——待处理文件数超过配置上限时中止。
//!
//! 黑名单分两类，语义不同，不要混用：
//!
//! - [`forbidden_trees`]：**连同所有子目录**一起禁止。用于系统目录，
//!   因为它们的任何子目录都不该被演练碰触。
//! - [`forbidden_selves`]：**只禁止目录本身**，允许其子目录。用于用户目录，
//!   因为「锁掉整个 `~/Documents`」是事故，但「锁掉 `~/Documents/演练目录`」
//!   恰恰是典型演练场景。

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

/// 系统目录：本身及所有子目录都禁止。
fn forbidden_trees() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();

    #[cfg(unix)]
    {
        for p in [
            "/System",
            "/Library",
            "/Applications",
            "/usr",
            "/bin",
            "/sbin",
            "/etc",
            "/var",
            "/private",
            "/opt",
            "/cores",
            "/dev",
            "/Volumes",
            "/Network",
        ] {
            v.push(PathBuf::from(p));
        }
        if let Some(home) = home_dir() {
            v.push(home.join("Library"));
            v.push(home.join(".ssh"));
            v.push(home.join(".gnupg"));
        }
    }

    #[cfg(windows)]
    {
        let sysroot = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
        v.push(PathBuf::from(&sysroot));
        for p in [
            "C:\\Program Files",
            "C:\\Program Files (x86)",
            "C:\\ProgramData",
            "C:\\$Recycle.Bin",
            "C:\\Recovery",
            "C:\\PerfLogs",
        ] {
            v.push(PathBuf::from(p));
        }
        if let Some(home) = home_dir() {
            v.push(home.join("AppData"));
        }
    }

    v
}

/// 用户关键目录：只禁止目录本身，允许其子目录。
fn forbidden_selves() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();

    // 文件系统根，永远不能作为演练根。
    #[cfg(unix)]
    v.push(PathBuf::from("/"));
    #[cfg(windows)]
    {
        for drive in 'C'..='Z' {
            v.push(PathBuf::from(format!("{drive}:\\")));
        }
    }

    if let Some(home) = home_dir() {
        v.push(home.clone());
        for sub in [
            "Desktop",
            "Documents",
            "Downloads",
            "Movies",
            "Music",
            "Pictures",
            "Public",
            "桌面",
            "文档",
            "下载",
            "图片",
        ] {
            v.push(home.join(sub));
        }
    }

    v
}

pub fn home_dir() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
}

/// 归一化路径以便比较：绝对化 + 解析符号链接。
fn normalize(p: &Path) -> PathBuf {
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(p))
            .unwrap_or_else(|_| p.to_path_buf())
    };
    // canonicalize 失败（例如目录还不存在）时退回绝对路径即可。
    abs.canonicalize().unwrap_or(abs)
}

#[cfg(windows)]
fn strip_verbatim(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
    PathBuf::from(s.to_string())
}

#[cfg(not(windows))]
fn strip_verbatim(p: &Path) -> PathBuf {
    p.to_path_buf()
}

/// `child` 是否就是 `parent`，或位于 `parent` 之下。
fn is_same_or_under(child: &Path, parent: &Path) -> bool {
    let child = strip_verbatim(child);
    let parent = strip_verbatim(parent);

    #[cfg(windows)]
    let (c, p) = (
        child.to_string_lossy().to_lowercase(),
        parent.to_string_lossy().to_lowercase(),
    );
    #[cfg(not(windows))]
    let (c, p) = (
        child.to_string_lossy().to_string(),
        parent.to_string_lossy().to_string(),
    );

    if c == p {
        return true;
    }
    let mut prefix = p.clone();
    if !prefix.ends_with(std::path::MAIN_SEPARATOR) {
        prefix.push(std::path::MAIN_SEPARATOR);
    }
    c.starts_with(&prefix)
}

/// 仅判断两个路径是否相同（不含子目录）。
fn is_same(a: &Path, b: &Path) -> bool {
    let a = strip_verbatim(a);
    let b = strip_verbatim(b);

    #[cfg(windows)]
    {
        a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
    }
    #[cfg(not(windows))]
    {
        a == b
    }
}

/// 校验演练根目录是否安全。不安全时返回带原因的 Err。
pub fn check_root(root: &Path) -> Result<()> {
    let root_abs = normalize(root);

    if !root_abs.is_absolute() {
        bail!("无法确定演练目录的绝对路径：{}", root.display());
    }

    if root_abs.parent().is_none() {
        bail!(
            "拒绝在文件系统根目录执行演练：{}\n演练应放在一个专用的空目录中。",
            root_abs.display()
        );
    }

    for bad in forbidden_trees() {
        let bad = normalize(&bad);
        if is_same_or_under(&root_abs, &bad) {
            bail!(
                "拒绝执行：演练目录 {} 位于系统目录 {} 之下。\n\
                 请把演练文件放到一个专用的演示目录（例如 ~/Desktop/演练演示）后重试。",
                root_abs.display(),
                bad.display()
            );
        }
    }

    for bad in forbidden_selves() {
        let bad = normalize(&bad);
        if is_same(&root_abs, &bad) {
            bail!(
                "拒绝执行：演练目录不能是 {} 本身（该目录范围过大）。\n\
                 请在其下新建一个专用子目录，例如 {}。",
                bad.display(),
                bad.join("演练演示").display()
            );
        }
    }

    Ok(())
}

/// 校验待处理文件数量是否在上限内。
pub fn check_file_count(count: usize, max: usize) -> Result<()> {
    if count > max {
        bail!(
            "待处理文件数 {count} 超过配置上限 {max}（lock.max_files）。\n\
             为避免误伤，已中止。确认无误请调高 config/drill.toml 中的该值后重新编译。"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 拒绝文件系统根目录() {
        #[cfg(unix)]
        assert!(check_root(Path::new("/")).is_err());
    }

    #[test]
    fn 拒绝家目录本身() {
        if let Some(home) = home_dir() {
            assert!(check_root(&home).is_err(), "家目录本身必须被拒绝");
        }
    }

    #[test]
    fn 拒绝系统目录及其子目录() {
        #[cfg(unix)]
        {
            assert!(check_root(Path::new("/System")).is_err());
            assert!(check_root(Path::new("/System/Library/CoreServices")).is_err());
        }
    }

    #[test]
    fn 允许家目录下的专用子目录() {
        if let Some(home) = home_dir() {
            let demo = home.join("Desktop").join("drill-demo-测试");
            // 该目录可能不存在，check_root 只需保证不因黑名单被拒。
            assert!(
                check_root(&demo).is_ok(),
                "家目录下的专用子目录应当被允许"
            );
            assert!(check_root(&home.join("Documents").join("演练")).is_ok());
        }
    }

    #[test]
    fn 拒绝家目录下的关键目录本身() {
        if let Some(home) = home_dir() {
            assert!(check_root(&home.join("Documents")).is_err());
            assert!(check_root(&home.join("Desktop")).is_err());
        }
    }

    #[test]
    fn 拒绝超量文件() {
        assert!(check_file_count(10, 5).is_err());
        assert!(check_file_count(5, 5).is_ok());
    }

    #[test]
    fn 前缀比较不会误判同名前缀目录() {
        // /System 与 /SystemX 是不同目录，不应互相匹配。
        assert!(!is_same_or_under(
            Path::new("/SystemX/foo"),
            Path::new("/System")
        ));
        assert!(is_same_or_under(
            Path::new("/System/Library"),
            Path::new("/System")
        ));
    }
}
