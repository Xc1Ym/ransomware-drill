//! HTML 模板渲染。
//!
//! 网页模板采用 `{{KEY}}` 双花括号占位符：HTML/CSS/JS 里本来就有大量单层花括号，
//! 用双括号可以避免和 `{}` 语法撞车。
//!
//! 列表类配置（快捷方式、搜索结果、邮件列表……）统一转成 JSON 数组嵌进页面，
//! 由页面里的脚本渲染，这样配置里增删条目不需要改动 HTML 结构。

use crate::config::{Config, FolderMail, SearchResult, Shortcut};

pub const INDEX_HTML: &str = include_str!("../../../web/index.html");
pub const BROWSER_HTML: &str = include_str!("../../../web/browser.html");
pub const DOWNLOAD_HTML: &str = include_str!("../../../web/download.html");
pub const SEARCH_HTML: &str = include_str!("../../../web/search.html");
pub const MAIL_HTML: &str = include_str!("../../../web/mail.html");

/// 把一个字符串转义成可以安全嵌进 `<script>` 里 JS 字符串字面量的形式（含首尾引号）。
///
/// 文案里可能有换行、引号，甚至 `</script>`，不转义会撑破字符串或截断脚本块。
fn json_str(s: &str) -> String {
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

/// 字符串数组 → JSON 数组。
fn json_str_array(items: &[String]) -> String {
    let parts: Vec<String> = items.iter().map(|s| json_str(s)).collect();
    format!("[{}]", parts.join(","))
}

/// 转义成可安全放进 JS **单引号**字面量的内容（不含引号）。
///
/// 页面里大量用 `'{{KEY}}'` 这种写法，配置文案一旦含单引号或换行就会把脚本撑破，
/// 所以这类值不能直接原样嵌入。
fn js_sq(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\'' => out.push_str("\\'"),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '<' => out.push_str("\\u003C"),
            '&' => out.push_str("\\u0026"),
            c => out.push(c),
        }
    }
    out
}

fn shortcuts_json(cfg: &Config) -> String {
    let parts: Vec<String> = cfg
        .browser
        .shortcut
        .iter()
        .map(|s: &Shortcut| {
            format!(
                "{{name:{},icon:{},color:{},link:{},highlight:{},dashed:{}}}",
                json_str(&cfg.expand(&s.name)),
                json_str(&s.icon),
                json_str(&s.color),
                json_str(&cfg.expand(&s.link)),
                s.highlight,
                s.dashed
            )
        })
        .collect();
    format!("[{}]", parts.join(","))
}

fn search_results_json(cfg: &Config) -> String {
    let parts: Vec<String> = cfg
        .search
        .result
        .iter()
        .map(|r: &SearchResult| {
            format!(
                "{{favicon:{},site:{},url:{},title:{},snippet:{}}}",
                json_str(&r.favicon),
                json_str(&cfg.expand(&r.site)),
                json_str(&cfg.expand(&r.url)),
                json_str(&cfg.expand(&r.title)),
                json_str(&cfg.expand(&r.snippet))
            )
        })
        .collect();
    format!("[{}]", parts.join(","))
}

fn mail_folder_json(cfg: &Config) -> String {
    let parts: Vec<String> = cfg
        .mail
        .folder
        .iter()
        .map(|m: &FolderMail| {
            format!(
                "{{sender:{},addr:{},time:{},subject:{},preview:{},body:{}}}",
                json_str(&cfg.expand(&m.sender)),
                json_str(&cfg.expand(&m.sender_email)),
                json_str(&m.time),
                json_str(&cfg.expand(&m.subject)),
                json_str(&cfg.expand(&m.preview)),
                json_str(&cfg.expand(m.body.trim()))
            )
        })
        .collect();
    format!("[{}]", parts.join(","))
}

/// 把模板里的 `{{KEY}}` 替换为配置中的值。
pub fn render(template: &str, cfg: &Config) -> String {
    let web = &cfg.web;

    let pairs: Vec<(&str, String)> = vec![
        // ---- 通用 ----
        ("ORG", cfg.organization.name.clone()),
        ("SHORT_NAME", cfg.organization.short_name.clone()),
        ("DRILL_CODE", cfg.organization.drill_code.clone()),
        ("DOMAIN", cfg.identity.domain.clone()),
        ("EXTERNAL_DOMAIN", cfg.identity.external_domain.clone()),
        ("DOWNLOAD_NAME", web.download_name.clone()),
        ("VERSION", web.download_version.clone()),
        ("SIZE", web.download_size.clone()),
        ("COUNT", web.download_count.clone()),
        ("FAKE_DOMAIN", js_sq(&web.fake_domain)),
        ("EMAIL", cfg.expand(&cfg.popup.email)),
        ("BITCOIN", cfg.popup.bitcoin.clone()),
        ("AMOUNT", cfg.popup.amount.clone()),
        ("POPUP_TITLE", cfg.popup.title.clone()),
        ("HEADLINE", cfg.expand(&cfg.popup.headline)),
        ("EXT", cfg.lock.extension.clone()),
        // ---- 入口页 ----
        ("PORTAL_TITLE", cfg.expand(&cfg.portal.title)),
        ("PORTAL_SUBTITLE", cfg.expand(&cfg.portal.subtitle)),
        ("CARD_DOWNLOAD_TITLE", cfg.expand(&cfg.portal.card_download_title)),
        ("CARD_DOWNLOAD_DESC", cfg.expand(&cfg.portal.card_download_desc)),
        ("CARD_MAIL_TITLE", cfg.expand(&cfg.portal.card_mail_title)),
        ("CARD_MAIL_DESC", cfg.expand(&cfg.portal.card_mail_desc)),
        ("RECOVERY_TITLE", cfg.expand(&cfg.portal.recovery_title)),
        ("RECOVERY_DESC", cfg.expand(&cfg.portal.recovery_desc)),
        ("RECOVERY_BUTTON", cfg.expand(&cfg.portal.recovery_button)),
        // ---- 仿浏览器 ----
        ("BROWSER_TAB_TITLE", cfg.expand(&cfg.browser.tab_title)),
        ("BROWSER_ADDRESS", cfg.expand(&cfg.browser.address)),
        ("BROWSER_SEARCH_PH", cfg.expand(&cfg.browser.search_placeholder)),
        ("BROWSER_LOGO_TEXT", {
            let t = cfg.expand(&cfg.browser.logo_text);
            if t.trim().is_empty() {
                cfg.organization.short_name.clone()
            } else {
                t
            }
        }),
        ("BROWSER_SHORTCUTS_JSON", shortcuts_json(cfg)),
        // ---- 仿搜索结果页 ----
        ("SEARCH_BRAND", cfg.expand(&cfg.search.brand)),
        (
            "SEARCH_DEFAULT_KEYWORD",
            js_sq(&cfg.expand(&cfg.search.default_keyword)),
        ),
        ("SEARCH_STATS", cfg.expand(&cfg.search.stats)),
        ("SEARCH_TABS_JSON", json_str_array(&cfg.search.tabs)),
        ("SEARCH_PAGER_JSON", json_str_array(&cfg.search.pager)),
        ("SEARCH_RELATED_JSON", json_str_array(
            &cfg.search
                .related
                .iter()
                .map(|s| cfg.expand(s))
                .collect::<Vec<_>>(),
        )),
        ("SEARCH_RESULTS_JSON", search_results_json(cfg)),
        // ---- 邮件 ----
        ("MAIL_BRAND", cfg.expand(&cfg.mail.brand)),
        ("MAIL_SEARCH_PH", cfg.expand(&cfg.mail.search_placeholder)),
        ("MAIL_COMPOSE_LABEL", cfg.expand(&cfg.mail.compose_label)),
        ("MAIL_OWNER", cfg.expand(&cfg.mail.owner)),
        ("MAIL_OWNER_EMAIL", cfg.expand(&cfg.mail.owner_email)),
        ("MAIL_NAV_JSON", json_str_array(&cfg.mail.nav_main)),
        (
            "MAIL_NAV_GROUP_TITLE",
            js_sq(&cfg.expand(&cfg.mail.nav_group_title)),
        ),
        ("MAIL_NAV_GROUP_JSON", json_str_array(&cfg.mail.nav_group)),
        ("MAIL_SUBJECT", js_sq(&cfg.expand(&cfg.mail.subject))),
        ("MAIL_PREVIEW", js_sq(&cfg.expand(&cfg.mail.preview))),
        ("MAIL_SENDER_NAME", js_sq(&cfg.expand(&cfg.mail.sender_name))),
        ("MAIL_SENDER_EMAIL", js_sq(&cfg.expand(&cfg.mail.sender_email))),
        ("MAIL_TIME", js_sq(&cfg.expand(&cfg.mail.time))),
        ("MAIL_ATTACHMENT", js_sq(&cfg.expand(&cfg.mail.attachment))),
        (
            "MAIL_ATTACHMENT_SIZE",
            js_sq(&cfg.expand(&cfg.mail.attachment_size)),
        ),
        ("MAIL_BODY_JSON", json_str(&cfg.expand(cfg.mail.body.trim()))),
        ("MAIL_FOLDER_JSON", mail_folder_json(cfg)),
        // ---- 下载页 ----
        ("WEB_BRAND", cfg.expand(&web.brand)),
        ("WEB_NAV_JSON", json_str_array(&web.nav)),
        ("WEB_RATING", web.download_rating.clone()),
        ("WEB_UPDATED", web.download_updated.clone()),
        ("WEB_TAGLINE", cfg.expand(&web.download_tagline)),
        ("WEB_INTRO", cfg.expand(&web.intro)),
        ("WEB_FEATURES_JSON", json_str_array(
            &web.features.iter().map(|s| cfg.expand(s)).collect::<Vec<_>>(),
        )),
        ("WEB_SCREENSHOTS_JSON", json_str_array(&web.screenshots)),
        ("WEB_INSTALL_JSON", json_str_array(
            &web.install_steps
                .iter()
                .map(|s| cfg.expand(s))
                .collect::<Vec<_>>(),
        )),
        ("CHROME_NAME", cfg.expand(&web.chrome_name)),
        ("CHROME_TAGLINE", cfg.expand(&web.chrome_tagline)),
        ("CHROME_INTRO", cfg.expand(&web.chrome_intro)),
        ("CHROME_FEATURES_JSON", json_str_array(
            &web.chrome_features
                .iter()
                .map(|s| cfg.expand(s))
                .collect::<Vec<_>>(),
        )),
        ("CHROME_SCREENSHOTS_JSON", json_str_array(&web.chrome_screenshots)),
    ];

    let mut out = template.to_string();
    for (key, value) in pairs {
        out = out.replace(&format!("{{{{{key}}}}}"), &value);
    }
    out
}

/// 渲染 `index.html`（演练场景入口页）。
pub fn index_html(cfg: &Config) -> String {
    render(INDEX_HTML, cfg)
}

/// 渲染 `browser.html`（仿浏览器新标签页）。
pub fn browser_html(cfg: &Config) -> String {
    render(BROWSER_HTML, cfg)
}

/// 渲染 `download.html`（仿软件下载站）。
pub fn download_html(cfg: &Config) -> String {
    render(DOWNLOAD_HTML, cfg)
}

/// 渲染 `search.html`（仿搜索引擎结果页）。
pub fn search_html(cfg: &Config) -> String {
    render(SEARCH_HTML, cfg)
}

/// 渲染 `mail.html`（仿邮箱钓鱼页面）。
pub fn mail_html(cfg: &Config) -> String {
    render(MAIL_HTML, cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;

    /// 渲染后不应残留任何未替换的占位符。
    fn assert_no_placeholder(name: &str, html: &str) {
        if let Some(pos) = html.find("{{") {
            let snippet: String = html[pos..].chars().take(50).collect();
            panic!("{name} 中存在未替换的占位符：{snippet}");
        }
    }

    #[test]
    fn 五个页面都不残留占位符() {
        let cfg = config::load().unwrap();
        assert_no_placeholder("index.html", &index_html(&cfg));
        assert_no_placeholder("browser.html", &browser_html(&cfg));
        assert_no_placeholder("search.html", &search_html(&cfg));
        assert_no_placeholder("download.html", &download_html(&cfg));
        assert_no_placeholder("mail.html", &mail_html(&cfg));
    }

    #[test]
    fn 域名占位符会展开到邮箱与网址() {
        let mut cfg = config::load().unwrap();
        cfg.identity.domain = "test-corp.cn".into();
        cfg.identity.external_domain = "phish-example.net".into();
        cfg.mail.owner_email = "zhangsan@{domain}".into();
        cfg.mail.sender_email = "hr@{external_domain}".into();

        let html = mail_html(&cfg);
        assert!(html.contains("zhangsan@test-corp.cn"), "内部邮箱域名应跟随配置");
        assert!(html.contains("hr@phish-example.net"), "外部域名应跟随配置");
    }

    #[test]
    fn 嵌套占位符能被完整展开() {
        let cfg = config::load().unwrap();
        // 配置里 popup.email 写的是 decrypt@{external_domain}
        let out = cfg.expand(&cfg.popup.email);
        assert!(!out.contains('{'), "嵌套引用必须展开干净，实际得到：{out}");
        assert!(out.contains(&cfg.identity.external_domain));
    }

    #[test]
    fn 列表配置会渲染成_json_数组() {
        let mut cfg = config::load().unwrap();
        cfg.search.related = vec!["甲".into(), "乙".into()];
        let html = search_html(&cfg);
        assert!(html.contains(r#"["甲","乙"]"#));
    }

    #[test]
    fn 邮件正文会被安全地转义进脚本() {
        let out = json_str("第一行\n第二行 \"引号\" </script> & <b>");
        assert!(out.starts_with('"') && out.ends_with('"'));
        assert!(out.contains("\\n"), "换行必须转义，否则会撑破 JS 字符串");
        assert!(!out.contains("</script>"), "不能让正文截断脚本块");
        assert!(!out.contains('\n'), "结果里不能有真实换行");
    }

    #[test]
    fn css_的大括号不受影响() {
        let cfg = config::load().unwrap();
        let html = render("body { color: red; } — {{ORG}}", &cfg);
        assert!(html.contains("body { color: red; }"));
    }
}
