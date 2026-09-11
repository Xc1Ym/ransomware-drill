//! 中文字体探测。
//!
//! 不内嵌字体（一个中文字体动辄十几 MB，会让演练程序变得笨重且引起怀疑），
//! 改为在运行时从系统里找。找不到时给出明确的错误提示和候选清单。

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

/// 按优先级排列的候选字体：`(路径, TTC 内的索引)`。
///
/// macOS 上优先用「冬青黑体」——它是黑体，视觉上与勒索弹窗的观感最接近。
pub fn candidates() -> Vec<(&'static str, u32)> {
    #[cfg(target_os = "macos")]
    {
        vec![
            ("/System/Library/Fonts/Hiragino Sans GB.ttc", 0),
            ("/System/Library/Fonts/STHeiti Medium.ttc", 0),
            ("/System/Library/Fonts/STHeiti Light.ttc", 0),
            ("/System/Library/Fonts/Supplemental/Songti.ttc", 0),
            ("/Library/Fonts/Arial Unicode.ttf", 0),
        ]
    }

    #[cfg(windows)]
    {
        vec![
            ("C:\\Windows\\Fonts\\msyh.ttc", 0),
            ("C:\\Windows\\Fonts\\msyhbd.ttc", 0),
            ("C:\\Windows\\Fonts\\simhei.ttf", 0),
            ("C:\\Windows\\Fonts\\simsun.ttc", 0),
        ]
    }

    #[cfg(not(any(target_os = "macos", windows)))]
    {
        vec![
            ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 0),
            ("/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc", 0),
        ]
    }
}

/// 已加载的字体：保留原始字节，供 `ab_glyph` 借用。
pub struct CjkFont {
    pub path: PathBuf,
    /// TTC 集合内的索引；普通 TTF 为 0。
    pub index: u32,
    pub data: Vec<u8>,
}

impl CjkFont {
    /// 构造一个 `ab_glyph` 字体视图。
    pub fn as_font(&self) -> Result<ab_glyph::FontRef<'_>> {
        ab_glyph::FontRef::try_from_slice_and_index(&self.data, self.index)
            .map_err(|e| anyhow::anyhow!("字体文件无法解析（{}）：{e}", self.path.display()))
    }
}

/// 加载中文字体：优先使用配置里显式指定的路径，否则按 [`candidates`] 依次探测。
pub fn load(override_path: Option<&Path>) -> Result<CjkFont> {
    if let Some(p) = override_path {
        let data = std::fs::read(p)
            .with_context(|| format!("读取配置指定的字体失败：{}", p.display()))?;
        return Ok(CjkFont {
            path: p.to_path_buf(),
            index: 0,
            data,
        });
    }

    let mut tried: Vec<&str> = Vec::new();
    for (path, index) in candidates() {
        let p = PathBuf::from(path);
        if p.is_file() {
            if let Ok(data) = std::fs::read(&p) {
                return Ok(CjkFont {
                    path: p,
                    index,
                    data,
                });
            }
        }
        tried.push(path);
    }

    bail!(
        "未找到可用的中文字体，壁纸将无法渲染中文。\n已尝试以下路径：\n  {}\n\
         请在 config/drill.toml 的 [wallpaper] 段中设置 font_path 指向一个中文字体文件后重新编译。",
        tried.join("\n  ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 候选列表非空() {
        assert!(!candidates().is_empty());
    }

    #[test]
    fn 显式路径不存在时报错() {
        let r = load(Some(Path::new("/definitely/not/a/font.ttf")));
        assert!(r.is_err());
    }
}
