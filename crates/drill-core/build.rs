//! 构建脚本：声明「编译期嵌入的文件」也是依赖。
//!
//! `config/drill.toml` 与 `web/*.html` 都是通过 `include_str!` 嵌进二进制的。
//! 问题在于：**只改这些文件、不动任何 .rs 代码时，cargo 不一定会重新编译**，
//! 于是 `cargo run -p xtask -- package` 打出来的还是上一次的旧内容——
//! 改配置、改页面的效果都不会生效，而且很难察觉。
//!
//! 这里显式声明依赖，保证这些文件一改，相关 crate 必定重新编译。

use std::path::Path;

fn main() {
    // CARGO_MANIFEST_DIR = <workspace>/crates/drill-core
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    let tracked = [
        "config/drill.toml",
        "web/index.html",
        "web/browser.html",
        "web/search.html",
        "web/download.html",
        "web/mail.html",
    ];

    for rel in tracked {
        let p = root.join(rel);
        println!("cargo:rerun-if-changed={}", p.display());
    }
}
