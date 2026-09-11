//! 测试辅助。
//!
//! 沙箱刻意放在 workspace 的 `target/` 下，而**不是** `std::env::temp_dir()`：
//! 在 macOS 上系统临时目录是 `/var/folders/...`（符号链接到 `/private/var/...`），
//! 会被 [`crate::safety`] 的 `/private` 规则正确拦截。护栏是对的，让测试换个地方即可。

use std::path::{Path, PathBuf};

/// 建一个干净的测试沙箱目录。
pub fn sandbox(tag: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/drill-test");
    let dir = root.join(format!("{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("创建测试沙箱失败");
    dir
}

/// 删除沙箱。
pub fn cleanup(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}
