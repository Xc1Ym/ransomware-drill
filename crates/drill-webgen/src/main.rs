//! `drill-webgen` —— 按配置渲染演练用的网页。
//!
//! 把 `web/` 下的模板里的 `{{KEY}}` 占位符替换成 `config/drill.toml` 里的值，
//! 输出到指定目录。网页上不会残留任何占位符。

use anyhow::{Context, Result};
use drill_core::{config, template};
use std::path::PathBuf;

const HELP: &str = r#"drill-webgen —— 渲染演练用的网页

用法：
    drill-webgen [输出目录]

参数：
    输出目录    渲染结果写入的位置，默认为 dist

说明：
    会把 web/index.html 与 web/download.html 中的 {{占位符}}
    替换为 config/drill.toml 中配置的单位信息与文案。
"#;

fn main() {
    if let Err(e) = run() {
        eprintln!("[错误] {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let arg = std::env::args().nth(1);
    if matches!(arg.as_deref(), Some("-h") | Some("--help")) {
        println!("{HELP}");
        return Ok(());
    }

    let cfg = config::load()?;
    let dir = PathBuf::from(arg.unwrap_or_else(|| "dist".to_string()));

    std::fs::create_dir_all(&dir)
        .with_context(|| format!("创建输出目录失败：{}", dir.display()))?;

    let pages = [
        ("index.html", template::index_html(&cfg)),
        ("browser.html", template::browser_html(&cfg)),
        ("download.html", template::download_html(&cfg)),
        ("search.html", template::search_html(&cfg)),
        ("mail.html", template::mail_html(&cfg)),
    ];

    // 渲染后不应再有未替换的占位符，出现即说明模板里写错了键名。
    for (name, html) in &pages {
        if let Some(pos) = html.find("{{") {
            let snippet: String = html[pos..].chars().take(40).collect();
            anyhow::bail!("{name} 中存在未替换的占位符：{snippet}");
        }
    }

    println!("已生成网页（单位：{}）：", cfg.organization.name);
    for (name, html) in &pages {
        let path = dir.join(name);
        std::fs::write(&path, html)
            .with_context(|| format!("写入失败：{}", path.display()))?;
        println!("    {}", path.display());
    }

    Ok(())
}
