//! 勒索病毒应急演练套件 —— 核心库
//!
//! # 安全声明
//!
//! 本库用于**网络安全应急演练演示**，不是真实勒索软件。它的全部「加密」行为
//! 实际上只是给文件名追加一个后缀（[`lock`]），存在以下硬性保证：
//!
//! - **不读写文件内容**：只调用 `std::fs::rename`，文件内容一个字节都不会变；
//! - **不删除任何文件**：代码中不存在删除文件的路径；
//! - **不加密、不联网**：无任何加解密与网络代码；
//! - **全流程可逆**：执行前必须先落盘 manifest，再执行重命名，
//!   配套的 `drill-restorer` 可 100% 还原文件后缀与桌面壁纸；
//! - **系统目录护栏**：见 [`safety`]，命中黑名单直接中止。

pub mod config;
pub mod fonts;
pub mod lock;
pub mod manifest;
pub mod platform;
pub mod restore;
pub mod safety;
pub mod template;
pub mod walk;
pub mod wallpaper;

#[cfg(test)]
pub(crate) mod testutil;

pub use config::{load, Config};
