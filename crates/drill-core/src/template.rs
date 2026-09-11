//! HTML 模板渲染。
//!
//! 网页模板刻意采用 `{{KEY}}` 双花括号占位符：HTML/CSS/JS 里本来就有大量
//! 单层花括号，用双括号可以避免和 `{}` 语法撞车。

use crate::config::Config;

pub const INDEX_HTML: &str = include_str!("../../../web/index.html");
pub const DOWNLOAD_HTML: &str = include_str!("../../../web/download.html");

/// 把模板里的 `{{KEY}}` 替换为配置中的值。
pub fn render(template: &str, cfg: &Config) -> String {
    let pairs: [(&str, String); 14] = [
        ("ORG", cfg.organization.name.clone()),
        ("SHORT_NAME", cfg.organization.short_name.clone()),
        ("DRILL_CODE", cfg.organization.drill_code.clone()),
        ("SITE_TITLE", cfg.web.site_title.clone()),
        ("DOWNLOAD_NAME", cfg.web.download_name.clone()),
        ("VERSION", cfg.web.download_version.clone()),
        ("SIZE", cfg.web.download_size.clone()),
        ("COUNT", cfg.web.download_count.clone()),
        ("EMAIL", cfg.popup.email.clone()),
        ("BITCOIN", cfg.popup.bitcoin.clone()),
        ("AMOUNT", cfg.popup.amount.clone()),
        ("POPUP_TITLE", cfg.popup.title.clone()),
        ("HEADLINE", cfg.popup.headline.clone()),
        ("EXT", cfg.lock.extension.clone()),
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
    fn css_的大括号不受影响() {
        let cfg = config::load().unwrap();
        let html = render("body { color: red; } — {{ORG}}", &cfg);
        assert!(html.contains("body { color: red; }"));
    }
}
