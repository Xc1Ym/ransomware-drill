//! 恢复：还原文件后缀与桌面壁纸。
//!
//! 恢复路径有两条，优先用 manifest（精确），manifest 丢失时退回按后缀剥离（兜底）。
//!
//! 注意：本模块**刻意不做**[`crate::safety::check_root`] 校验。恢复是「救灾」操作，
//! 万一有人真的把演练跑到了不该跑的目录，恢复工具必须能够无条件把它救回来。

use crate::manifest::Manifest;
use crate::platform;
use crate::{config::Config, note, walk};
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct RestoreReport {
    pub root: PathBuf,
    pub restored: usize,
    /// manifest 中登记但磁盘上找不到的文件。
    pub missing: Vec<PathBuf>,
    pub failed: Vec<(PathBuf, String)>,
    /// 是否使用了 manifest（false 表示走了后缀剥离兜底）。
    pub used_manifest: bool,
    pub manifest_path: Option<PathBuf>,
    pub wallpaper_restored: Option<PathBuf>,
    pub wallpaper_error: Option<String>,
    /// 已清除的勒索信数量。
    pub notes_removed: usize,
    pub note_error: Option<String>,
    pub dry_run: bool,
}

/// 恢复 `root` 目录下的所有文件与桌面壁纸。
pub fn restore_dir(root: &Path, cfg: &Config, dry_run: bool) -> Result<RestoreReport> {
    let mut report = RestoreReport {
        root: root.to_path_buf(),
        dry_run,
        ..Default::default()
    };

    let manifest_path = walk::find_manifest(root);
    let pairs: Vec<(PathBuf, PathBuf)> = match &manifest_path {
        Some(p) => {
            let m = Manifest::read_from(p)?;
            report.used_manifest = true;
            report.manifest_path = Some(p.clone());

            if !dry_run {
                if let Some(wallpaper) = m.original_wallpaper.as_ref() {
                    match platform::set_wallpaper(wallpaper) {
                        Ok(()) => report.wallpaper_restored = Some(wallpaper.clone()),
                        Err(e) => report.wallpaper_error = Some(e.to_string()),
                    }
                }
                // 清除演练自己投放的勒索信（唯一会删除文件的一步）
                match note::remove_notes(&m.notes, cfg) {
                    Ok(n) => report.notes_removed = n,
                    Err(e) => report.note_error = Some(e.to_string()),
                }
            } else {
                report.wallpaper_restored = m.original_wallpaper.clone();
                report.notes_removed = m.notes.iter().filter(|p| p.is_file()).count();
            }

            m.entries
                .into_iter()
                .map(|e| (e.locked, e.original))
                .collect()
        }
        None => {
            // 兜底：没有 manifest 时，凡是带该后缀的文件都剥掉后缀还原。
            collect_suffixed(root, cfg.lock.recursive, &cfg.lock.extension)
        }
    };

    for (locked, original) in pairs {
        if !locked.exists() {
            report.missing.push(locked);
            continue;
        }
        if original.exists() {
            // 目标名已被占用，不覆盖，转为报告。
            report
                .failed
                .push((original, "同名文件已存在，已跳过以免覆盖".into()));
            continue;
        }
        if dry_run {
            report.restored += 1;
            continue;
        }
        match std::fs::rename(&locked, &original) {
            Ok(()) => report.restored += 1,
            Err(e) => report.failed.push((locked, e.to_string())),
        }
    }

    Ok(report)
}

/// 收集所有以 `suffix` 结尾的文件，返回 `(锁定路径, 还原后路径)`。
fn collect_suffixed(root: &Path, recursive: bool, suffix: &str) -> Vec<(PathBuf, PathBuf)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_symlink() {
                continue;
            }
            let path = entry.path();
            if ft.is_dir() {
                if recursive {
                    stack.push(path);
                }
                continue;
            }
            if !ft.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name == crate::manifest::FILE_NAME {
                continue;
            }
            if let Some(stripped) = name.strip_suffix(suffix) {
                if !stripped.is_empty() {
                    out.push((path.clone(), dir.join(stripped)));
                }
            }
        }
    }

    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use crate::lock;

    fn 建沙箱(tag: &str) -> PathBuf {
        crate::testutil::sandbox(&format!("restore-{tag}"))
    }

    #[test]
    fn 依据_manifest_完整还原() {
        let dir = 建沙箱("manifest");
        let content = b"important content";
        std::fs::write(dir.join("报告.txt"), content).unwrap();
        std::fs::create_dir_all(dir.join("子目录")).unwrap();
        std::fs::write(dir.join("子目录/data.bin"), b"bin").unwrap();

        let cfg = config::load().unwrap();
        let locked = lock::lock_dir(&dir, &cfg, false, None).unwrap();
        assert_eq!(locked.locked, 2);

        let report = restore_dir(&dir, &cfg, false).unwrap();
        assert!(report.used_manifest);
        assert_eq!(report.restored, 2);
        assert!(report.failed.is_empty());

        assert!(dir.join("报告.txt").exists());
        assert!(dir.join("子目录/data.bin").exists());
        assert_eq!(std::fs::read(dir.join("报告.txt")).unwrap(), content);

        crate::testutil::cleanup(&dir);
    }

    #[test]
    fn 无_manifest_时按后缀剥离兜底() {
        let dir = 建沙箱("fallback");
        // 直接造出「已锁定」状态，不放 manifest。
        std::fs::write(dir.join("a.txt.drill_locked"), b"a").unwrap();
        std::fs::write(dir.join("b.doc.drill_locked"), b"b").unwrap();

        let cfg = config::load().unwrap();
        let report = restore_dir(&dir, &cfg, false).unwrap();

        assert!(!report.used_manifest, "没有 manifest 时应走兜底路径");
        assert_eq!(report.restored, 2);
        assert!(dir.join("a.txt").exists());
        assert!(dir.join("b.doc").exists());

        crate::testutil::cleanup(&dir);
    }

    #[test]
    fn 不覆盖已存在的同名文件() {
        let dir = 建沙箱("noclobber");
        std::fs::write(dir.join("a.txt"), b"original").unwrap();
        std::fs::write(dir.join("a.txt.drill_locked"), b"locked").unwrap();

        let cfg = config::load().unwrap();
        let report = restore_dir(&dir, &cfg, false).unwrap();

        assert_eq!(report.restored, 0);
        assert_eq!(report.failed.len(), 1);
        assert_eq!(
            std::fs::read(dir.join("a.txt")).unwrap(),
            b"original",
            "已存在的文件不能被覆盖"
        );

        crate::testutil::cleanup(&dir);
    }

    #[test]
    fn dry_run_不改动文件() {
        let dir = 建沙箱("dryrun");
        std::fs::write(dir.join("a.txt"), b"a").unwrap();
        let cfg = config::load().unwrap();
        lock::lock_dir(&dir, &cfg, false, None).unwrap();

        let report = restore_dir(&dir, &cfg, true).unwrap();
        assert_eq!(report.restored, 1);
        assert!(
            dir.join("a.txt.drill_locked").exists(),
            "dry-run 不应真的改名"
        );

        crate::testutil::cleanup(&dir);
    }
}
