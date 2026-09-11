//! 「锁定」——即给文件名追加伪加密后缀。
//!
//! 真正的加密在这里**不存在**：全部操作只有一次 `std::fs::rename`，
//! 文件的字节内容不会被读取、修改或删除。

use crate::manifest::{Entry, Manifest};
use crate::{config::Config, note, safety, walk};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct LockReport {
    pub root: PathBuf,
    /// 扫描到的候选文件数。
    pub candidates: usize,
    /// 实际改名成功的数量。
    pub locked: usize,
    /// 因为已经锁过而跳过的数量。
    pub skipped: usize,
    pub failed: Vec<(PathBuf, String)>,
    pub manifest_path: Option<PathBuf>,
    /// 投放的勒索信数量。
    pub notes_written: usize,
    pub dry_run: bool,
}

/// 追加后缀：`报告.docx` -> `报告.docx.drill_locked`。
fn locked_path(original: &Path, suffix: &str) -> PathBuf {
    let mut s = original.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}

/// 锁定 `root` 目录。
///
/// `original_wallpaper` 会被记入 manifest，供恢复时还原桌面。
///
/// 顺序上**先写 manifest、再改名**：这样即便改名中途被打断，
/// 也已经有一份完整的「原文件名 -> 锁后文件名」映射可供恢复。
pub fn lock_dir(
    root: &Path,
    cfg: &Config,
    dry_run: bool,
    original_wallpaper: Option<PathBuf>,
) -> Result<LockReport> {
    safety::check_root(root)?;

    let candidates = walk::collect_files(
        root,
        cfg.lock.recursive,
        &cfg.effective_exclude(),
        &cfg.lock.extension,
    )?;

    safety::check_file_count(candidates.len(), cfg.lock.max_files)?;

    let mut report = LockReport {
        root: root.to_path_buf(),
        candidates: candidates.len(),
        dry_run,
        ..Default::default()
    };

    if dry_run {
        return Ok(report);
    }

    let suffix = cfg.lock.extension.clone();
    let manifest_path = Manifest::default_path(root);

    // 如果目录里已经有 manifest，说明之前跑过演练，此时必须**增量合并**而不是新建：
    // 否则重复运行时候选文件为 0，会用一个空 manifest 把上一次的恢复记录覆盖掉，
    // 导致文件被锁着却再也还原不回来。
    let mut manifest = match Manifest::read_from(&manifest_path) {
        Ok(mut old) => {
            // 保留上一次记录的原始壁纸。重复运行时读到的「当前壁纸」很可能
            // 已经是演练壁纸了，用它覆盖会导致永远还原不回真实壁纸。
            old.locked_at = chrono::Local::now().to_rfc3339();
            old
        }
        Err(_) => Manifest::new(
            cfg.organization.drill_code.clone(),
            cfg.organization.name.clone(),
            root.to_path_buf(),
            suffix.clone(),
            original_wallpaper,
        ),
    };

    // 登记全部候选（而非只登记成功项：宁可多记，不可漏记），
    // 并与已有条目去重合并。
    for p in &candidates {
        let entry = Entry {
            original: p.clone(),
            locked: locked_path(p, &suffix),
        };
        if !manifest.entries.iter().any(|e| e.locked == entry.locked) {
            manifest.entries.push(entry);
        }
    }
    manifest.entries.sort_by(|a, b| a.locked.cmp(&b.locked));

    manifest
        .write(&manifest_path)
        .context("写入 manifest 失败，已中止（不会在无记录的情况下锁定文件）")?;
    report.manifest_path = Some(manifest_path.clone());

    for entry in &manifest.entries {
        if entry.locked.exists() {
            report.skipped += 1;
            continue;
        }
        if !entry.original.exists() {
            report.skipped += 1;
            continue;
        }
        match std::fs::rename(&entry.original, &entry.locked) {
            Ok(()) => report.locked += 1,
            Err(e) => report
                .failed
                .push((entry.original.clone(), e.to_string())),
        }
    }

    // 在每个被锁定的目录里投放勒索信，并把路径补登记进 manifest——
    // 不登记的话恢复时清不掉，演练结束后会在目录里留下垃圾文件。
    if cfg.ransom_note.enabled {
        let dirs: std::collections::BTreeSet<PathBuf> = manifest
            .entries
            .iter()
            .filter_map(|e| e.original.parent().map(Path::to_path_buf))
            .collect();

        match note::write_notes(&dirs, cfg) {
            Ok(notes) => {
                report.notes_written = notes.len();
                // 同样用合并而非覆盖，避免把上一次投放的勒索信记录冲掉
                let mut changed = false;
                for n in notes {
                    if !manifest.notes.contains(&n) {
                        manifest.notes.push(n);
                        changed = true;
                    }
                }
                if changed {
                    manifest.notes.sort();
                    manifest
                        .write(&manifest_path)
                        .context("登记勒索信路径失败")?;
                }
            }
            Err(e) => report
                .failed
                .push((root.to_path_buf(), format!("投放勒索信失败：{e}"))),
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;

    fn 建沙箱(tag: &str) -> PathBuf {
        crate::testutil::sandbox(&format!("lock-{tag}"))
    }

    #[test]
    fn 改写后缀但内容不变() {
        let dir = 建沙箱("content");
        let original = b"important content";
        std::fs::write(dir.join("report.txt"), original).unwrap();

        let cfg = config::load().unwrap();
        let report = lock_dir(&dir, &cfg, false, None).unwrap();
        assert_eq!(report.locked, 1);

        let locked = dir.join("report.txt.drill_locked");
        assert!(locked.exists(), "文件应已改名");
        assert!(!dir.join("report.txt").exists(), "原文件名不应存在");
        assert_eq!(
            std::fs::read(&locked).unwrap(),
            original,
            "文件内容必须一个字节都没变"
        );

        crate::testutil::cleanup(&dir);
    }

    #[test]
    fn dry_run_不产生任何改动() {
        let dir = 建沙箱("dryrun");
        std::fs::write(dir.join("a.txt"), b"a").unwrap();

        let cfg = config::load().unwrap();
        let report = lock_dir(&dir, &cfg, true, None).unwrap();

        assert_eq!(report.candidates, 1);
        assert_eq!(report.locked, 0);
        assert!(dir.join("a.txt").exists());
        assert!(!dir.join("manifest.json").exists(), "dry-run 不应写 manifest");

        crate::testutil::cleanup(&dir);
    }

    #[test]
    fn 拒绝锁系统目录() {
        let cfg = config::load().unwrap();
        #[cfg(unix)]
        assert!(lock_dir(Path::new("/System"), &cfg, true, None).is_err());
    }

    #[test]
    fn 重复执行不丢失已有的恢复记录() {
        let dir = 建沙箱("merge");
        std::fs::write(dir.join("a.txt"), b"a").unwrap();

        let cfg = config::load().unwrap();
        lock_dir(&dir, &cfg, false, None).unwrap();

        let path = Manifest::default_path(&dir);
        let first = Manifest::read_from(&path).unwrap();
        assert_eq!(first.entries.len(), 1, "首次运行应登记 1 条");
        assert_eq!(first.notes.len(), 1, "首次运行应登记 1 封勒索信");

        // 再跑一次：此时候选文件为 0，但绝不能把已有记录冲成空 manifest，
        // 否则文件被锁着却再也还原不回来。
        lock_dir(&dir, &cfg, false, None).unwrap();

        let second = Manifest::read_from(&path).unwrap();
        assert_eq!(second.entries.len(), 1, "重复运行后锁定记录必须还在");
        assert_eq!(second.notes.len(), 1, "重复运行后勒索信记录必须还在");

        crate::testutil::cleanup(&dir);
    }

    #[test]
    fn 重复执行是幂等的() {
        let dir = 建沙箱("idem");
        std::fs::write(dir.join("a.txt"), b"a").unwrap();

        let cfg = config::load().unwrap();
        let first = lock_dir(&dir, &cfg, false, None).unwrap();
        assert_eq!(first.locked, 1);

        let second = lock_dir(&dir, &cfg, false, None).unwrap();
        assert_eq!(second.locked, 0, "第二次不应再改名");
        assert_eq!(second.candidates, 0, "已锁定文件不应被再次收集");

        crate::testutil::cleanup(&dir);
    }
}
