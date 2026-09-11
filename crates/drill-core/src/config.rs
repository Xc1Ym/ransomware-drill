//! 配置模型。
//!
//! `config/drill.toml` 通过 [`include_str!`] 在**编译期**嵌入二进制，
//! 因此修改配置后必须重新编译才能生效。这样做的目的是让整个套件
//! （弹窗、壁纸、下载页、恢复工具）始终读取同一份配置，不会出现
//! 「网页上是 A 单位、弹窗里是 B 单位」的错配。

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

/// 编译期嵌入的配置原文。
pub const RAW: &str = include_str!("../../../config/drill.toml");

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub organization: Organization,
    pub lock: LockConfig,
    pub wallpaper: WallpaperConfig,
    pub popup: PopupConfig,
    pub web: WebConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Organization {
    pub name: String,
    pub short_name: String,
    pub drill_code: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LockConfig {
    /// 伪加密后缀，形如 `.drill_locked`。
    pub extension: String,
    pub recursive: bool,
    pub max_files: usize,
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WallpaperConfig {
    pub width: u32,
    pub height: u32,
    pub background: String,
    pub title: String,
    pub subtitle: String,
    pub accent: String,
    pub subtitle_color: String,
    pub font_path: String,
    pub drill_notice: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PopupConfig {
    pub title: String,
    pub headline: String,
    pub email: String,
    pub bitcoin: String,
    pub amount: String,
    pub pay_raise_hours: i64,
    pub files_lost_hours: i64,
    pub body: String,
    pub show_drill_disclaimer: bool,
    pub disclaimer: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebConfig {
    pub site_title: String,
    pub download_name: String,
    pub download_version: String,
    pub download_size: String,
    pub download_count: String,
}

/// 解析编译期嵌入的配置，并做基本校验。
pub fn load() -> Result<Config> {
    let cfg: Config = toml::from_str(RAW).context("config/drill.toml 解析失败")?;
    cfg.validate()?;
    Ok(cfg)
}

impl Config {
    fn validate(&self) -> Result<()> {
        let ext = &self.lock.extension;
        if !ext.starts_with('.') {
            bail!("lock.extension 必须以 '.' 开头，当前为 {ext:?}");
        }
        // 后缀过长通常是配置写错了，而不是有意为之。
        if ext.len() > 32 {
            bail!("lock.extension 过长（{} 字符），疑似配置错误", ext.len());
        }
        if ext.trim_matches('.').is_empty() {
            bail!("lock.extension 不能只有点号，当前为 {ext:?}");
        }
        if self.lock.max_files == 0 {
            bail!("lock.max_files 必须大于 0");
        }
        if self.wallpaper.width == 0 || self.wallpaper.height == 0 {
            bail!("wallpaper 分辨率不能为 0");
        }
        if self.organization.name.trim().is_empty() {
            bail!("organization.name 不能为空，请先填写演练单位名称");
        }
        Ok(())
    }

    /// 展开文案中的占位符，如 `{org}`、`{amount}`。
    ///
    /// 未知占位符原样保留，便于在模板中排查拼写错误。
    pub fn expand(&self, text: &str) -> String {
        let pairs: [(&str, String); 11] = [
            ("org", self.organization.name.clone()),
            ("short_name", self.organization.short_name.clone()),
            ("drill_code", self.organization.drill_code.clone()),
            ("email", self.popup.email.clone()),
            ("amount", self.popup.amount.clone()),
            ("bitcoin", self.popup.bitcoin.clone()),
            ("pay_raise_hours", self.popup.pay_raise_hours.to_string()),
            ("files_lost_hours", self.popup.files_lost_hours.to_string()),
            ("download_name", self.web.download_name.clone()),
            ("version", self.web.download_version.clone()),
            ("size", self.web.download_size.clone()),
        ];
        let mut out = text.to_string();
        for (key, value) in pairs {
            out = out.replace(&format!("{{{key}}}"), &value);
        }
        out
    }

    /// 弹窗最终展示的正文（含可选的演练声明）。
    pub fn popup_body(&self) -> String {
        let mut body = self.expand(self.popup.body.trim());
        if self.popup.show_drill_disclaimer && !self.popup.disclaimer.trim().is_empty() {
            body.push_str("\n\n");
            body.push_str(self.expand(self.popup.disclaimer.trim()).as_str());
        }
        body
    }

    /// 壁纸渲染用的字体文件路径（可能为空，交由 [`crate::fonts`] 探测）。
    pub fn font_override(&self) -> Option<PathBuf> {
        let p = self.wallpaper.font_path.trim();
        if p.is_empty() {
            None
        } else {
            Some(PathBuf::from(p))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 内置配置可以正常解析() {
        let cfg = load().expect("内置 config/drill.toml 应当可以解析");
        assert!(!cfg.organization.name.is_empty());
        assert!(cfg.lock.extension.starts_with('.'));
    }

    #[test]
    fn 占位符会被展开() {
        let cfg = load().unwrap();
        let out = cfg.expand("单位：{org}，金额 {amount}");
        assert!(out.contains(&cfg.organization.name));
        assert!(out.contains(&cfg.popup.amount));
        assert!(!out.contains("{org}"));
    }

    #[test]
    fn 未知占位符原样保留() {
        let cfg = load().unwrap();
        assert_eq!(cfg.expand("{不存在的占位符}"), "{不存在的占位符}");
    }

    #[test]
    fn 演练声明可追加到正文() {
        let mut cfg = load().unwrap();
        cfg.popup.show_drill_disclaimer = false;
        let without = cfg.popup_body();
        cfg.popup.show_drill_disclaimer = true;
        let with = cfg.popup_body();
        assert!(with.len() > without.len());
    }
}
