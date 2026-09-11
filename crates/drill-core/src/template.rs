//! HTML 模板渲染。
//!
//! 网页模板刻意采用 `{{KEY}}` 双花括号占位符：HTML/CSS/JS 里本来就有大量
//! 单层花括号，用双括号可以避免和 `{}` 语法撞车。

use crate::config::Config;

pub const INDEX_HTML: &str = include_str!("../../../web/index.html");
pub const BROWSER_HTML: &str = include_str!("../../../web/browser.html");
pub const DOWNLOAD_HTML: &str = include_str!("../../../web/download.html");
pub const SEARCH_HTML: &str = include_str!("../../../web/search.html");
pub const MAIL_HTML: &str = include_str!("../../../web/mail.html");

/// 把一个字符串转义成可以安全嵌进 `<script>` 里 JS 字符串字面量的形式（含首尾引号）。
///
/// 邮件正文是多行文本，直接塞进 JS 会把模板字符串撑破；同时要顺手转义
/// `<` `>` `&`，避免正文里出现 `</script>` 之类的内容截断脚本块。
fn json_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '<' => out.push_str("\\u003C"),
            '>' => out.push_str("\\u003E"),
            '&' => out.push_str("\\u0026"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// 把模板里的 `{{KEY}}` 替换为配置中的值。
pub fn render(template: &str, cfg: &Config) -> String {
    let pairs: [(&str, String); 30] = [
        ("ORG", cfg.organization.name.clone()),
        ("SHORT_NAME", cfg.organization.short_name.clone()),
        ("DRILL_CODE", cfg.organization.drill_code.clone()),
        ("SITE_TITLE", cfg.web.site_title.clone()),
        ("DOWNLOAD_NAME", cfg.web.download_name.clone()),
        ("VERSION", cfg.web.download_version.clone()),
        ("SIZE", cfg.web.download_size.clone()),
        ("COUNT", cfg.web.download_count.clone()),
        ("FAKE_DOMAIN", cfg.web.fake_domain.clone()),
        ("EMAIL", cfg.popup.email.clone()),
        ("BITCOIN", cfg.popup.bitcoin.clone()),
        ("AMOUNT", cfg.popup.amount.clone()),
        ("POPUP_TITLE", cfg.popup.title.clone()),
        ("HEADLINE", cfg.popup.headline.clone()),
        ("EXT", cfg.lock.extension.clone()),
        // 演练入口页
        ("PORTAL_TITLE", cfg.expand(&cfg.portal.title)),
        ("PORTAL_SUBTITLE", cfg.expand(&cfg.portal.subtitle)),
        ("CARD_DOWNLOAD_TITLE", cfg.portal.card_download_title.clone()),
        ("CARD_DOWNLOAD_DESC", cfg.expand(&cfg.portal.card_download_desc)),
        ("CARD_MAIL_TITLE", cfg.portal.card_mail_title.clone()),
        ("CARD_MAIL_DESC", cfg.expand(&cfg.portal.card_mail_desc)),
        // 钓鱼邮件
        ("MAIL_OWNER", cfg.mail.owner.clone()),
        ("MAIL_OWNER_EMAIL", cfg.mail.owner_email.clone()),
        ("MAIL_SUBJECT", cfg.expand(&cfg.mail.subject)),
        ("MAIL_SENDER_NAME", cfg.mail.sender_name.clone()),
        ("MAIL_SENDER_EMAIL", cfg.mail.sender_email.clone()),
        ("MAIL_TIME", cfg.mail.time.clone()),
        ("MAIL_ATTACHMENT", cfg.mail.attachment.clone()),
        ("MAIL_ATTACHMENT_SIZE", cfg.mail.attachment_size.clone()),
        (
            "MAIL_BODY_JSON",
            json_string_literal(&cfg.expand(cfg.mail.body.trim())),
        ),
    ];

    let mut out = template.to_string();
    for (key, value) in pairs {
        out = out.replace(&format!("{{{{{key}}}}}"), &value);
    }
    out
}

/// 渲染 `index.html`。
pub fn index_html(cfg: &Config) -> String {
    render(INDEX_HTML, cfg)
}

/// 渲染 `download.html`。
pub fn download_html(cfg: &Config) -> String {
    render(DOWNLOAD_HTML, cfg)
}

/// 渲染 `search.html`（仿搜索引擎结果页）。
pub fn search_html(cfg: &Config) -> String {
    render(SEARCH_HTML, cfg)
}

/// 渲染 `browser.html`（仿浏览器新标签页，下载勒索场景的入口）。
pub fn browser_html(cfg: &Config) -> String {
    render(BROWSER_HTML, cfg)
}

/// 渲染 `mail.html`（仿邮箱钓鱼页面）。
pub fn mail_html(cfg: &Config) -> String {
    render(MAIL_HTML, cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;

    #[test]
    fn 占位符全部被替换() {
        let cfg = config::load().unwrap();
        let html = index_html(&cfg);
        assert!(!html.contains("{{ORG}}"));
        assert!(html.contains(&cfg.organization.name));
    }

    #[test]
    fn 入口页占位符全部被替换() {
        let cfg = config::load().unwrap();
        let html = index_html(&cfg);
        assert!(!html.contains("{{"), "index.html 里存在未替换的占位符");
        assert!(html.contains(&cfg.organization.name));
        assert!(html.contains(&cfg.portal.card_download_title));
        assert!(html.contains(&cfg.portal.card_mail_title));
    }

    #[test]
    fn 邮件页占位符全部被替换() {
        let cfg = config::load().unwrap();
        let html = mail_html(&cfg);
        assert!(!html.contains("{{"), "mail.html 里存在未替换的占位符");
        assert!(html.contains(&cfg.mail.subject));
        assert!(html.contains(&cfg.mail.attachment));
    }

    #[test]
    fn 邮件正文会被安全地转义进脚本() {
        let out = json_string_literal("第一行\n第二行 \"引号\" </script> & <b>");
        assert!(out.starts_with('"') && out.ends_with('"'));
        assert!(out.contains("\\n"), "换行必须转义，否则会撑破 JS 字符串");
        assert!(!out.contains("</script>"), "不能让正文截断脚本块");
        assert!(!out.contains('\n'), "结果里不能有真实换行");
    }

    #[test]
    fn 搜索结果页占位符全部被替换() {
        let cfg = config::load().unwrap();
        let html = search_html(&cfg);
        assert!(
            !html.contains("{{"),
            "search.html 里存在未替换的占位符"
        );
        assert!(html.contains(&cfg.web.fake_domain));
    }

    #[test]
    fn css_的大括号不受影响() {
        let cfg = config::load().unwrap();
        let html = render("body { color: red; } — {{ORG}}", &cfg);
        assert!(html.contains("body { color: red; }"));
    }
}
