//! 演练清单（manifest）。
//!
//! manifest 是「已经锁了哪些文件」的唯一权威记录，也是恢复的依据。
//! 锁定流程**必须先成功写入 manifest，再执行任何重命名**——否则宁可中止，
//! 也绝不能出现「文件被锁但没记录」的状态。

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "manifest.json";

/// manifest 结构版本，为将来的格式变更留出兼容判断的余地。
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub drill_code: String,
    pub organization: String,
    pub suite_version: String,
    /// 锁定发生时间（RFC3339 本地时间），用于演练记录。
    pub locked_at: String,
    /// 演练根目录。
    pub root: PathBuf,
    /// 使用的伪加密后缀。
    pub suffix: String,
    /// 锁定前的桌面壁纸路径，恢复时用于还原。
    pub original_wallpaper: Option<PathBuf>,
    /// 本次演练投放的勒索信路径，恢复时一并清除。
    /// 老版本 manifest 没有这个字段，因此允许缺省。
    #[serde(default)]
    pub notes: Vec<PathBuf>,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    /// 原始路径。
    pub original: PathBuf,
    /// 锁定后的路径。
    pub locked: PathBuf,
}

impl Manifest {
    pub fn new(
        drill_code: String,
        organization: String,
        root: PathBuf,
        suffix: String,
        original_wallpaper: Option<PathBuf>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            drill_code,
            organization,
            suite_version: env!("CARGO_PKG_VERSION").to_string(),
            locked_at: chrono::Local::now().to_rfc3339(),
            root,
            suffix,
            original_wallpaper,
            notes: Vec::new(),
            entries: Vec::new(),
        }
    }

    pub fn default_path(root: &Path) -> PathBuf {
        root.join(FILE_NAME)
    }

    /// 原子写入 manifest：先写临时文件再 rename，避免写到一半留下损坏文件。
    pub fn write(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self).context("序列化 manifest 失败")?;

        let tmp = path.with_extension("json.tmp");
        {
            let mut f = std::fs::File::create(&tmp)
                .with_context(|| format!("创建 manifest 失败：{}", tmp.display()))?;
            f.write_all(json.as_bytes())
                .with_context(|| format!("写入 manifest 失败：{}", tmp.display()))?;
            // 落盘后再 rename，确保掉电时不会只剩半截文件。
            f.sync_all().context("manifest 落盘失败")?;
        }
        std::fs::rename(&tmp, path)
            .with_context(|| format!("保存 manifest 失败：{}", path.display()))?;
        Ok(())
    }

    /// 从指定文件读取 manifest。
    pub fn read_from(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("读取 manifest 失败：{}", path.display()))?;
        let m: Self = serde_json::from_str(&text)
            .with_context(|| format!("解析 manifest 失败：{}", path.display()))?;
        Ok(m)
    }

    /// 成功锁定的文件数量。
    pub fn locked_count(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn 临时路径(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("drill-manifest-{tag}-{}.json", std::process::id()))
    }

    #[test]
    fn 写入后可完整读回() {
        let path = 临时路径("roundtrip");
        let mut m = Manifest::new(
            "2026-测试-001".into(),
            "测试单位".into(),
            PathBuf::from("/tmp/demo"),
            ".drill_locked".into(),
            Some(PathBuf::from("/tmp/old-wallpaper.png")),
        );
        m.entries.push(Entry {
            original: PathBuf::from("/tmp/demo/报告.docx"),
            locked: PathBuf::from("/tmp/demo/报告.docx.drill_locked"),
        });

        m.write(&path).unwrap();
        let back = Manifest::read_from(&path).unwrap();

        assert_eq!(back.drill_code, "2026-测试-001");
        assert_eq!(back.organization, "测试单位");
        assert_eq!(back.suffix, ".drill_locked");
        assert_eq!(back.entries.len(), 1);
        assert_eq!(back.entries[0].original, m.entries[0].original);
        assert_eq!(
            back.original_wallpaper,
            Some(PathBuf::from("/tmp/old-wallpaper.png"))
        );
        assert_eq!(back.schema_version, SCHEMA_VERSION);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn 写入后不残留临时文件() {
        let path = 临时路径("notmp");
        let m = Manifest::new(
            "D".into(),
            "O".into(),
            PathBuf::from("/tmp/demo"),
            ".locked".into(),
            None,
        );
        m.write(&path).unwrap();
        assert!(!path.with_extension("json.tmp").exists());
        let _ = std::fs::remove_file(&path);
    }
}
