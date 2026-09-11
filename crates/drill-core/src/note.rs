//! 勒索信：在每个被锁定的目录里投放一份说明文件。
//!
//! 这是勒索软件的标志性特征，也是演练中参演人员最直接的「发现点」——
//! 真实事件里，很多人正是先看到桌面或文件夹里多出来的这个文件才意识到中招。

use crate::config::Config;
use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// 生成某个目录下勒索信的正文。
pub fn render(cfg: &Config, dir: &Path) -> String {
    let line = "─".repeat(58);

    let mut s = String::new();
    s.push_str("╔══════════════════════════════════════════════════════════╗\n");
    s.push_str("║                  您的文件已被加密！                      ║\n");
    s.push_str("╚══════════════════════════════════════════════════════════╝\n\n");

    s.push_str(&cfg.note_body());
    s.push_str("\n\n");

    s.push_str(&line);
    s.push_str("\n  联系方式\n");
    s.push_str(&line);
    s.push('\n');
    s.push_str(&format!("  邮箱      ：{}\n", cfg.popup.email));
    s.push_str(&format!("  比特币地址：{}\n", cfg.popup.bitcoin));
    s.push_str(&format!("  赎金金额  ：{}\n", cfg.popup.amount));
    s.push_str(&format!(
        "  付款时限  ：{} 小时内付款，逾期金额翻倍；{} 小时后密钥销毁\n",
        cfg.popup.pay_raise_hours, cfg.popup.files_lost_hours
    ));
    s.push('\n');
    s.push_str(&format!("  个人识别码：{}\n", identifier(cfg, dir)));
    s.push_str("  （请在邮件中附上此识别码，以便我们为您提供解密服务）\n\n");

    s.push_str(&line);
    s.push('\n');
    s.push_str(&format!("  所在目录：{}\n", dir.display()));
    s.push_str(&format!(
        "  加密时间：{}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    ));
    s.push_str(&format!("  伪加密后缀：{}\n", cfg.lock.extension));
    s.push_str(&line);
    s.push('\n');

    s
}

/// 个人识别码：由前缀、演练编号与目录路径派生，便于复盘时对位到具体目录。
fn identifier(cfg: &Config, dir: &Path) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    dir.hash(&mut hasher);
    format!(
        "{}-{}-{:08X}",
        cfg.ransom_note.id_prefix.trim(),
        cfg.organization.drill_code,
        hasher.finish() as u32
    )
}

/// 在给定目录集合中投放勒索信，返回实际写入的文件路径。
pub fn write_notes(dirs: &BTreeSet<PathBuf>, cfg: &Config) -> Result<Vec<PathBuf>> {
    if !cfg.ransom_note.enabled {
        return Ok(Vec::new());
    }

    let mut written = Vec::new();
    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        let path = dir.join(cfg.ransom_note.filename.trim());
        // 前置 UTF-8 BOM：Windows 记事本靠它判断编码，否则中文会显示成乱码。
        let content = format!("\u{FEFF}{}", render(cfg, dir));
        std::fs::write(&path, content)
            .with_context(|| format!("写入勒索信失败：{}", path.display()))?;
        written.push(path);
    }
    Ok(written)
}

/// 删除演练投放的勒索信，返回删除数量。
///
/// 这是整个代码库里**唯一**的删除操作，因此上了双重保险：
/// 只删除 manifest 中登记过的路径，且文件名必须与当前配置完全一致——
/// 任何一项对不上就跳过，宁可留下文件也不误删。
pub fn remove_notes(notes: &[PathBuf], cfg: &Config) -> Result<usize> {
    let expected = cfg.ransom_note.filename.trim();
    let mut removed = 0;

    for path in notes {
        let name_matches = path
            .file_name()
            .map(|n| n.to_string_lossy() == expected)
            .unwrap_or(false);
        if !name_matches {
            continue;
        }
        if path.is_file() {
            std::fs::remove_file(path)
                .with_context(|| format!("删除勒索信失败：{}", path.display()))?;
            removed += 1;
        }
    }

    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;

    #[test]
    fn 勒索信包含关键信息() {
        let cfg = config::load().unwrap();
        let text = render(&cfg, Path::new("/tmp/demo"));

        assert!(text.contains(&cfg.organization.name));
        assert!(text.contains(&cfg.popup.email));
        assert!(text.contains(&cfg.popup.bitcoin));
        assert!(text.contains(&cfg.popup.amount));
        assert!(text.contains("个人识别码"));
        assert!(text.contains(&cfg.ransom_note.id_prefix));
    }

    #[test]
    fn 同一目录的识别码稳定不同目录不同() {
        let cfg = config::load().unwrap();
        let a1 = identifier(&cfg, Path::new("/tmp/a"));
        let a2 = identifier(&cfg, Path::new("/tmp/a"));
        let b = identifier(&cfg, Path::new("/tmp/b"));
        assert_eq!(a1, a2, "同一目录的识别码应当稳定");
        assert_ne!(a1, b, "不同目录的识别码应当不同");
    }

    #[test]
    fn 只删除文件名匹配的勒索信() {
        let cfg = config::load().unwrap();
        let dir = crate::testutil::sandbox("note-remove");

        let real = dir.join(cfg.ransom_note.filename.trim());
        std::fs::write(&real, "note").unwrap();
        // 一个「不该被删」的同目录文件
        let bystander = dir.join("重要数据.txt");
        std::fs::write(&bystander, "keep me").unwrap();

        let removed = remove_notes(&[real.clone(), bystander.clone()], &cfg).unwrap();

        assert_eq!(removed, 1, "只应删除勒索信本身");
        assert!(!real.exists());
        assert!(bystander.exists(), "同目录的其它文件绝不能被删除");

        crate::testutil::cleanup(&dir);
    }

    #[test]
    fn 文件名对不上时跳过() {
        let cfg = config::load().unwrap();
        let dir = crate::testutil::sandbox("note-skip");
        let disguised = dir.join("伪装成别的东西.txt");
        std::fs::write(&disguised, "not our note").unwrap();

        let removed = remove_notes(&[disguised.clone()], &cfg).unwrap();
        assert_eq!(removed, 0);
        assert!(disguised.exists(), "文件名不匹配时不应删除");

        crate::testutil::cleanup(&dir);
    }
}
