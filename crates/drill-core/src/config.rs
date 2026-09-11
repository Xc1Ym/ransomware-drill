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
    /// 以下各段在老版本配置文件里可能不存在，因此都允许缺省。
    #[serde(default)]
    pub identity: IdentityConfig,
    #[serde(default)]
    pub ransom_note: RansomNoteConfig,
    #[serde(default)]
    pub portal: PortalConfig,
    #[serde(default)]
    pub browser: BrowserConfig,
    #[serde(default)]
    pub search: SearchConfig,
    #[serde(default)]
    pub mail: MailConfig,
}

/// 域名与身份：演练素材里的邮箱、网址都由这两个域名派生。
#[derive(Debug, Clone, Deserialize)]
pub struct IdentityConfig {
    /// 内部域名：内部邮箱后缀、内网示例网址。
    pub domain: String,
    /// 外部域名：钓鱼邮件发件人用它，伪装成外部机构。
    pub external_domain: String,
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            domain: "example.com".to_string(),
            external_domain: "example-corp.com".to_string(),
        }
    }
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

/// 钓鱼邮件的内容与邮箱界面文案。
#[derive(Debug, Clone, Deserialize)]
pub struct MailConfig {
    // ---- 界面 ----
    pub brand: String,
    pub search_placeholder: String,
    pub compose_label: String,
    pub owner: String,
    pub owner_email: String,
    pub nav_main: Vec<String>,
    pub nav_group_title: String,
    pub nav_group: Vec<String>,

    // ---- 钓鱼邮件 ----
    pub subject: String,
    pub preview: String,
    pub sender_name: String,
    pub sender_email: String,
    pub time: String,
    /// 附件显示名。推荐用双扩展名（如 `表格.xlsx.exe`），这是真实钓鱼的常见手法。
    pub attachment: String,
    pub attachment_size: String,
    pub body: String,

    /// 收件箱里的其它装饰邮件。
    #[serde(default)]
    pub folder: Vec<FolderMail>,
}

/// 收件箱里的一封普通邮件（装饰用，不含附件）。
#[derive(Debug, Clone, Deserialize)]
pub struct FolderMail {
    pub sender: String,
    pub sender_email: String,
    pub time: String,
    pub subject: String,
    pub preview: String,
    #[serde(default)]
    pub body: String,
}

impl Default for MailConfig {
    fn default() -> Self {
        Self {
            brand: "QQ邮箱".to_string(),
            search_placeholder: "搜索邮件".to_string(),
            compose_label: "写信".to_string(),
            owner: "张三".to_string(),
            owner_email: "zhangsan@{domain}".to_string(),
            nav_main: vec![
                "收件箱".into(),
                "星标邮件".into(),
                "群邮件".into(),
                "草稿箱".into(),
                "已发送".into(),
                "已删除".into(),
                "垃圾箱".into(),
            ],
            nav_group_title: "我的文件夹".to_string(),
            nav_group: vec!["工作文档".into(), "会议纪要".into(), "归档".into()],
            subject: "【人事部】2026年度工资调整明细表，请查收".to_string(),
            preview: "各位同事，2026年度工资调整明细已核算完毕，请查收附件…".to_string(),
            sender_name: "人事部-王经理".to_string(),
            sender_email: "hr@{external_domain}".to_string(),
            time: "今天 09:24".to_string(),
            attachment: "2026年度工资调整明细表.xlsx.exe".to_string(),
            attachment_size: "8.4 MB".to_string(),
            body: "各位同事：\n\n请查收附件并核对个人信息。\n\n人事部".to_string(),
            folder: Vec::new(),
        }
    }
}

/// 仿浏览器新标签页（下载勒索场景的入口）。
#[derive(Debug, Clone, Deserialize)]
pub struct BrowserConfig {
    pub tab_title: String,
    pub address: String,
    pub search_placeholder: String,
    /// 圆形标识里的文字，留空则回退到单位简称。
    #[serde(default)]
    pub logo_text: String,
    #[serde(default)]
    pub shortcut: Vec<Shortcut>,
}

/// 一个新标签页快捷方式磁贴。
#[derive(Debug, Clone, Deserialize)]
pub struct Shortcut {
    pub name: String,
    /// 图标名，可选：download mail calendar doc cloud grid approve shield settings folder chart add
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub color: String,
    /// 点击后跳转的地址，留空表示不可点击（纯装饰）。
    #[serde(default)]
    pub link: String,
    /// 是否做高亮样式（用于主推的那个入口）。
    #[serde(default)]
    pub highlight: bool,
    /// 是否用虚线边框样式（用于「添加」那类磁贴）。
    #[serde(default)]
    pub dashed: bool,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            tab_title: "新标签页".to_string(),
            address: "https://www.{domain}/".to_string(),
            search_placeholder: "搜索或输入网址".to_string(),
            logo_text: String::new(),
            shortcut: Vec::new(),
        }
    }
}

/// 仿搜索引擎结果页。
#[derive(Debug, Clone, Deserialize)]
pub struct SearchConfig {
    pub brand: String,
    pub default_keyword: String,
    pub stats: String,
    #[serde(default)]
    pub tabs: Vec<String>,
    #[serde(default)]
    pub pager: Vec<String>,
    #[serde(default)]
    pub related: Vec<String>,
    /// 装饰性的搜索结果条目（置顶的「广告」陷阱由程序按关键词生成）。
    #[serde(default)]
    pub result: Vec<SearchResult>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchResult {
    /// 左侧圆圈里的那个字。
    #[serde(default)]
    pub favicon: String,
    pub site: String,
    pub url: String,
    pub title: String,
    pub snippet: String,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            brand: "搜索".to_string(),
            default_keyword: "软件 官方下载".to_string(),
            stats: "找到约 2,140,000 条结果 （用时 0.38 秒）".to_string(),
            tabs: vec![
                "全部".into(),
                "图片".into(),
                "视频".into(),
                "新闻".into(),
                "地图".into(),
            ],
            pager: vec!["1".into(), "2".into(), "3".into(), "…".into(), "下一页".into()],
            related: Vec::new(),
            result: Vec::new(),
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
    /// 下载站品牌名与顶部导航。
    #[serde(default = "default_brand")]
    pub brand: String,
    #[serde(default)]
    pub nav: Vec<String>,

    pub download_name: String,
    pub download_version: String,
    pub download_size: String,
    pub download_count: String,
    #[serde(default = "default_rating")]
    pub download_rating: String,
    #[serde(default)]
    pub download_updated: String,
    #[serde(default)]
    pub download_tagline: String,

    /// 伪装成浏览器下载页（?as=chrome）时替换用的信息。
    #[serde(default = "default_chrome_name")]
    pub chrome_name: String,
    #[serde(default)]
    pub chrome_tagline: String,
    #[serde(default)]
    pub chrome_intro: String,
    #[serde(default)]
    pub chrome_features: Vec<String>,
    #[serde(default)]
    pub chrome_screenshots: Vec<String>,

    #[serde(default)]
    pub intro: String,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub screenshots: Vec<String>,
    #[serde(default)]
    pub install_steps: Vec<String>,

    /// 搜索结果页「广告位」展示的仿冒域名（纯展示，不会真的访问）。
    #[serde(default = "default_fake_domain")]
    pub fake_domain: String,
}

fn default_brand() -> String {
    "软件中心".to_string()
}
fn default_rating() -> String {
    "4.9".to_string()
}
fn default_chrome_name() -> String {
    "Chrome 浏览器".to_string()
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

    /// 展开文案中的占位符，如 `{org}`、`{domain}`、`{amount}`。
    ///
    /// 会做多轮替换：配置里允许出现嵌套引用（例如 `email = "decrypt@{external_domain}"`），
    /// 只跑一轮的话内层占位符会残留下来。未知占位符原样保留，
    /// 便于在页面上直接看出是哪个键名写错了。
    pub fn expand(&self, text: &str) -> String {
        let pairs: [(&str, String); 13] = [
            ("org", self.organization.name.clone()),
            ("short_name", self.organization.short_name.clone()),
            ("drill_code", self.organization.drill_code.clone()),
            ("domain", self.identity.domain.clone()),
            ("external_domain", self.identity.external_domain.clone()),
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
        for _ in 0..4 {
            let before = out.clone();
            for (key, value) in &pairs {
                out = out.replace(&format!("{{{key}}}"), value);
            }
            if out == before {
                break;
            }
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
