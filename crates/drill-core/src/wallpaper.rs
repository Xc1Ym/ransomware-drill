//! 勒索壁纸渲染：纯黑背景 + 红色中文标语。
//!
//! 壁纸同样**不内嵌任何图片资源**，而是在运行时用系统字体现场绘制，
//! 这样单位名称、标语、字号都能跟着配置走。

use crate::config::Config;
use crate::fonts;
use anyhow::{bail, Context, Result};
use image::{Rgb, RgbImage};
use imageproc::drawing::{draw_text_mut, text_size};
use std::path::{Path, PathBuf};

/// 解析 `#RRGGBB` 形式的颜色。
pub fn parse_hex_color(s: &str) -> Result<Rgb<u8>> {
    let raw = s.trim().trim_start_matches('#');
    if raw.len() != 6 || !raw.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("颜色格式应为 #RRGGBB，当前为 {s:?}");
    }
    let r = u8::from_str_radix(&raw[0..2], 16)?;
    let g = u8::from_str_radix(&raw[2..4], 16)?;
    let b = u8::from_str_radix(&raw[4..6], 16)?;
    Ok(Rgb([r, g, b]))
}

/// 在 `cx` 处水平居中绘制一行文字，返回该行占用的高度。
fn draw_centered(
    img: &mut RgbImage,
    color: Rgb<u8>,
    cx: i32,
    y: i32,
    size: f32,
    font: &impl ab_glyph::Font,
    text: &str,
) -> i32 {
    let scale = ab_glyph::PxScale::from(size);
    let (w, h) = text_size(scale, font, text);
    let x = cx - (w as i32) / 2;
    draw_text_mut(img, color, x, y, scale, font, text);
    h as i32
}

/// 计算一行文字在给定字号下的像素宽度。
fn text_width(size: f32, font: &impl ab_glyph::Font, text: &str) -> u32 {
    text_size(ab_glyph::PxScale::from(size), font, text).0
}

/// 找出让 `text` 宽度不超过 `max_width` 的最大字号（从 `size` 开始只减不增）。
fn fit_size(mut size: f32, min_size: f32, max_width: u32, font: &impl ab_glyph::Font, text: &str) -> f32 {
    while size > min_size && text_width(size, font, text) > max_width {
        size -= size * 0.05;
    }
    size
}

/// 渲染壁纸并写入 `out`。
pub fn render(cfg: &Config, out: &Path) -> Result<PathBuf> {
    let w = cfg.wallpaper.width;
    let h = cfg.wallpaper.height;

    let bg = parse_hex_color(&cfg.wallpaper.background)?;
    let accent = parse_hex_color(&cfg.wallpaper.accent)?;
    let subtitle_color = parse_hex_color(&cfg.wallpaper.subtitle_color)?;

    let font_data = fonts::load(cfg.font_override().as_deref())?;
    let font = font_data.as_font()?;

    let mut img = RgbImage::from_pixel(w, h, bg);

    let max_text_width = (w as f32 * 0.88) as u32;

    // 主标题
    let title_text = cfg.expand(&cfg.wallpaper.title);
    let title_size = fit_size(w as f32 * 0.085, 24.0, max_text_width, &font, &title_text);

    // 副标题（可能多行）
    let subtitle_raw = cfg.expand(&cfg.wallpaper.subtitle);
    let subtitle_lines: Vec<&str> = subtitle_raw
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    let subtitle_size = subtitle_lines
        .iter()
        .map(|l| {
            fit_size(
                w as f32 * 0.038,
                16.0,
                max_text_width,
                &font,
                l,
            )
        })
        .fold(f32::MAX, f32::min)
        .min(w as f32 * 0.038);
    let subtitle_size = if subtitle_size == f32::MAX {
        0.0
    } else {
        subtitle_size
    };

    // 整体垂直居中：先算总高度，再决定起始 y
    let title_h = if title_text.trim().is_empty() {
        0
    } else {
        text_size(ab_glyph::PxScale::from(title_size), &font, &title_text).1 as i32
    };
    let line_h = (subtitle_size * 1.5) as i32;
    let gap = (h as f32 * 0.045) as i32;
    let block_h = title_h + if subtitle_lines.is_empty() { 0 } else { gap } + line_h * subtitle_lines.len() as i32;

    let mut y = (h as i32 - block_h) / 2;
    let cx = (w / 2) as i32;

    if title_h > 0 {
        draw_centered(&mut img, accent, cx, y, title_size, &font, &title_text);
        y += title_h;
    }
    if !subtitle_lines.is_empty() {
        y += gap;
        for line in &subtitle_lines {
            draw_centered(&mut img, subtitle_color, cx, y, subtitle_size, &font, line);
            y += line_h;
        }
    }

    // 可选的演练标识，放在右下角，字号很小、颜色低调
    let notice = cfg.wallpaper.drill_notice.trim();
    if !notice.is_empty() {
        let notice_size = (w as f32 * 0.014).max(12.0);
        let scale = ab_glyph::PxScale::from(notice_size);
        let (nw, nh) = text_size(scale, &font, notice);
        let nx = w as i32 - nw as i32 - (w as f32 * 0.02) as i32;
        let ny = h as i32 - nh as i32 - (h as f32 * 0.02) as i32;
        draw_text_mut(&mut img, Rgb([90, 90, 90]), nx, ny, scale, &font, notice);
    }

    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建输出目录失败：{}", parent.display()))?;
    }
    img.save(out)
        .with_context(|| format!("保存壁纸失败：{}", out.display()))?;

    Ok(out.to_path_buf())
}

/// 渲染壁纸到临时目录，返回文件路径。
pub fn render_to_temp(cfg: &Config) -> Result<PathBuf> {
    let dir = std::env::temp_dir().join("drill-wallpaper");
    let out = dir.join("wallpaper-drill.png");
    render(cfg, &out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;

    #[test]
    fn 解析颜色() {
        assert_eq!(parse_hex_color("#000000").unwrap(), Rgb([0, 0, 0]));
        assert_eq!(parse_hex_color("FF0000").unwrap(), Rgb([255, 0, 0]));
        assert_eq!(parse_hex_color("#1a2B3c").unwrap(), Rgb([26, 43, 60]));
        assert!(parse_hex_color("红色").is_err());
        assert!(parse_hex_color("#FFF").is_err());
    }

    #[test]
    fn 渲染出尺寸正确的壁纸() {
        let mut cfg = config::load().unwrap();
        // 用较小尺寸让测试跑得快些
        cfg.wallpaper.width = 640;
        cfg.wallpaper.height = 400;

        let out = std::env::temp_dir().join(format!("drill-wp-test-{}.png", std::process::id()));
        let path = render(&cfg, &out).expect("壁纸应当渲染成功");

        let img = image::open(&path).unwrap();
        assert_eq!(img.width(), 640);
        assert_eq!(img.height(), 400);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn 字号会自动缩小以适配宽度() {
        let cfg = config::load().unwrap();
        let font_data = fonts::load(cfg.font_override().as_deref()).unwrap();
        let font = font_data.as_font().unwrap();
        let long = "这是一个非常非常长的标题用于测试自动缩小字号的逻辑是否生效";
        let size = fit_size(400.0, 16.0, 500, &font, long);
        assert!(size < 400.0, "长文本应当触发字号缩小");
        assert!(text_width(size, &font, long) <= 500);
    }
}
