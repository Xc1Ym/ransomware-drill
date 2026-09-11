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
    /// 勒索信配置。旧版本配置文件里没有这一段，因此允许缺省。
    #[serde(default)]
    pub ransom_note: RansomNoteConfig,
    /// 演练入口页（选择场景）。
    #[serde(default)]
    pub portal: PortalConfig,
    /// 钓鱼邮件（仿邮箱场景）。
    #[serde(default)]
    pub mail: MailConfig,
}

/// 演练入口页：让参演者选择走哪条「中招路径」。
#[derive(Debug, Clone, Deserialize)]
pub struct PortalConfig {
    /// 主页大标题，支持 `{org}` 占位符。
    pub title: String,
    pub subtitle: String,
    pub card_download_title: String,
    pub card_download_desc: String,
    pub card_mail_title: String,
    pub card_mail_desc: String,
}

impl Default for PortalConfig {
    fn default() -> Self {
        Self {
            title: "{org}勒索病毒应急演练".to_string(),
            subtitle: "请选择演练场景".to_string(),
            card_download_title: "文件下载勒索".to_string(),
            card_download_desc: "模拟员工从搜索结果进入仿冒下载站，下载并运行看似正常的程序，实际中招。"
                .to_string(),
            card_mail_title: "邮件勒索".to_string(),
            card_mail_desc: "模拟员工收到一封伪装成人事通知的钓鱼邮件，点开附件后中招。".to_string(),
        }
    }
}

/// 钓鱼邮件的内容。
#[derive(Debug, Clone, Deserialize)]
pub struct MailConfig {
    /// 邮箱页右上角显示的收件人。
    pub owner: String,
    pub owner_email: String,
    pub subject: String,
    pub sender_name: String,
    pub sender_email: String,
    pub time: String,
    /// 附件显示名。推荐用双扩展名（如 `表格.xlsx.exe`），这是真实钓鱼的常见手法。
    pub attachment: String,
    pub attachment_size: String,
    pub body: String,
}

impl Default for MailConfig {
    fn default() -> Self {
        Self {
            owner: "张三".to_string(),
            owner_email: "zhangsan@example.com".to_string(),
            subject: "【人事部】2026年度工资调整明细表，请查收".to_string(),
            sender_name: "人事部-王经理".to_string(),
            sender_email: "hr@example-corp.com".to_string(),
            time: "今天 09:24".to_string(),
            attachment: "2026年度工资调整明细表.xlsx.exe".to_string(),
            attachment_size: "8.4 MB".to_string(),
            body: "各位同事：\n\n请查收附件并核对个人信息。\n\n人事部".to_string(),
        }
    }
}

/// 勒索信：在每个被锁定的目录里投放一份说明文件。
///
/// 这是勒索软件的标志性特征，也是演练中参演人员最直接的「发现点」。
#[derive(Debug, Clone, Deserialize)]
pub struct RansomNoteConfig {
    pub enabled: bool,
    /// 勒索信文件名。
    pub filename: String,
    /// 「个人识别码」的前缀，用于演练复盘时对位。
    pub id_prefix: String,
    /// 正文；留空则复用弹窗正文。
    #[serde(default)]
    pub body: String,
}

impl Default for RansomNoteConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            filename: "如何恢复你的文件.txt".to_string(),
            id_prefix: "WNCRY".to_string(),
            body: String::new(),
        }
    }
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
    /// 搜索结果页「广告位」展示的仿冒域名（纯展示，不会真的访问）。
    #[serde(default = "default_fake_domain")]
    pub fake_domain: String,
}

fn default_fake_domain() -> String {
    "downlod-center.com".to_string()
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
        if self.ransom_note.enabled && self.ransom_note.filename.trim().is_empty() {
            bail!("ransom_note.filename 不能为空");
        }
        Ok(())
    }

    /// 遍历时要跳过的名字：配置里的 `lock.exclude`，再加上演练自己产生的文件。
    ///
    /// 必须排除勒索信，否则重复运行演练时它会被当成普通文件再锁一遍。
    pub fn effective_exclude(&self) -> Vec<String> {
        let mut v = self.lock.exclude.clone();
        if self.ransom_note.enabled {
            let f = self.ransom_note.filename.trim();
            if !f.is_empty() && !v.iter().any(|x| x == f) {
                v.push(f.to_string());
            }
        }
        v
    }

    /// 勒索信的正文：优先用专属配置，否则复用弹窗正文。
    pub fn note_body(&self) -> String {
        let raw = if self.ransom_note.body.trim().is_empty() {
            self.popup.body.trim()
        } else {
            self.ransom_note.body.trim()
        };
        let mut body = self.expand(raw);
        if self.popup.show_drill_disclaimer && !self.popup.disclaimer.trim().is_empty() {
            body.push_str("\n\n");
            body.push_str(self.expand(self.popup.disclaimer.trim()).as_str());
        }
        body
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
