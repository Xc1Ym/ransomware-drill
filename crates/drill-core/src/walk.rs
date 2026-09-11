//! 目录遍历：收集待锁定的文件。
//!
//! 规则刻意保守——**不跟随符号链接**（避免顺着链接锁到目录树外面去），
//! 只处理普通文件，并跳过配置中的排除项与已经带后缀的文件。

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// 收集 `root` 下所有应当被锁定的文件。
///
/// - `recursive = false` 时只处理 `root` 这一层；
/// - 跳过名字命中 `exclude` 的目录与文件；
/// - 跳过符号链接（含指向文件的链接）；
/// - 跳过已经以 `suffix` 结尾的文件，保证重复执行是幂等的。
pub fn collect_files(
    root: &Path,
    recursive: bool,
    exclude: &[String],
    suffix: &str,
) -> Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            // 单个目录读不了（权限等）不应让整个演练失败，跳过即可。
            Err(_) => continue,
        };

        for entry in entries {
            let entry = entry.with_context(|| format!("读取目录项失败：{}", dir.display()))?;
            let name = entry.file_name().to_string_lossy().to_string();

            if is_excluded(&name, exclude) {
                continue;
            }

            let file_type = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };

            // 符号链接一律跳过：避免沿着链接把范围扩大到目录树之外。
            if file_type.is_symlink() {
                continue;
            }

            let path = entry.path();

            if file_type.is_dir() {
                if recursive {
                    stack.push(path);
                }
                continue;
            }

            if file_type.is_file() {
                // 已经锁过的文件不再重复处理。
                if name.ends_with(suffix) {
                    continue;
                }
                out.push(path);
            }
        }
    }

    // 排序让输出与 manifest 稳定可复现，便于演练记录比对。
    out.sort();
    Ok(out)
}

fn is_excluded(name: &str, exclude: &[String]) -> bool {
    exclude.iter().any(|e| e == name)
}

/// 在 `root` 下查找已有的 manifest。
pub fn find_manifest(root: &Path) -> Option<PathBuf> {
    let direct = root.join(crate::manifest::FILE_NAME);
    if direct.is_file() {
        return Some(direct);
    }
    // 兜底：manifest 可能被放在子目录里。
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
            if ft.is_dir() {
                stack.push(entry.path());
            } else if entry.file_name().to_string_lossy() == crate::manifest::FILE_NAME {
                return Some(entry.path());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn 建测试目录(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("drill-walk-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn 递归收集文件并跳过排除项() {
        let dir = 建测试目录("recursive");
        std::fs::write(dir.join("a.txt"), b"a").unwrap();
        std::fs::write(dir.join("b.doc"), b"b").unwrap();
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("sub/c.txt"), b"c").unwrap();
        std::fs::create_dir_all(dir.join("target")).unwrap();
        std::fs::write(dir.join("target/d.txt"), b"d").unwrap();

        let files = collect_files(&dir, true, &["target".to_string()], ".locked").unwrap();
        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(names.contains(&"a.txt".to_string()));
        assert!(names.contains(&"b.doc".to_string()));
        assert!(names.contains(&"c.txt".to_string()));
        assert!(
            !names.contains(&"d.txt".to_string()),
            "排除目录 target 中的文件不应被收集"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 非递归只处理当前层() {
        let dir = 建测试目录("flat");
        std::fs::write(dir.join("top.txt"), b"t").unwrap();
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("sub/deep.txt"), b"d").unwrap();

        let files = collect_files(&dir, false, &[], ".locked").unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("top.txt"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 已锁定文件不重复收集() {
        let dir = 建测试目录("idempotent");
        std::fs::write(dir.join("x.txt.locked"), b"x").unwrap();
        std::fs::write(dir.join("y.txt"), b"y").unwrap();

        let files = collect_files(&dir, true, &[], ".locked").unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("y.txt"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
