//! `xtask` —— 演练套件的一键打包入口。
//!
//! ```text
//! cargo run -p xtask -- build       # 编译 release 二进制
//! cargo run -p xtask -- package     # 编译 + 渲染网页 + 生成壁纸 → 组装 dist/
//! cargo run -p xtask -- sandbox     # 造一批测试样本文件，用于试跑
//! cargo run -p xtask -- wallpaper   # 只生成一张壁纸，方便预览效果
//! ```

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

const HELP: &str = r#"xtask —— 勒索病毒应急演练套件打包工具

用法：
    cargo run -p xtask -- <命令>

命令：
    build        编译 release 版本的 drill-locker 与 drill-restorer
    package      完整打包：编译 + 渲染网页 + 生成壁纸 + 组装 dist/
    sandbox      在 target/drill-sandbox 下造一批测试样本文件
    wallpaper    只生成一张壁纸到 dist/wallpaper.png，方便预览
    help         显示本帮助

换单位演练的流程：
    1. 修改 config/drill.toml 中的单位名称与文案
    2. cargo run -p xtask -- package
    3. 把生成的 dist/ 整个目录拷贝到演练机器上使用
"#;

fn workspace_root() -> PathBuf {
    // crates/xtask -> crates -> workspace 根
    let raw = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    // 规范化掉 `../`，否则打印出来的路径会很难看
    raw.canonicalize().unwrap_or(raw)
}

fn main() {
    if let Err(e) = run() {
        eprintln!("[错误] {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("help");

    match cmd {
        "help" | "-h" | "--help" => {
            println!("{HELP}");
            Ok(())
        }
        "build" => build_release(),
        "package" => package(),
        "sandbox" => {
            let dir = args
                .get(1)
                .map(PathBuf::from)
                .unwrap_or_else(|| workspace_root().join("target/drill-sandbox"));
            sandbox(&dir)
        }
        "wallpaper" => {
            let out = workspace_root().join("dist/wallpaper.png");
            wallpaper_only(&out)
        }
        other => bail!("无法识别的命令：{other}\n运行 cargo run -p xtask -- help 查看用法。"),
    }
}

fn build_release() -> Result<()> {
    println!("==> 编译 release 版本…");
    let status = Command::new(env!("CARGO"))
        .args(["build", "--release"])
        .current_dir(workspace_root())
        .status()
        .context("执行 cargo build 失败，请确认 cargo 在 PATH 中")?;
    if !status.success() {
        bail!("cargo build --release 失败");
    }
    Ok(())
}

/// 组装可直接拷贝到演练机器使用的 `dist/` 目录。
fn package() -> Result<()> {
    let root = workspace_root();
    let cfg = drill_core::config::load()?;
    let dist = root.join("dist");

    build_release()?;

    println!("==> 组装 dist/ …");
    if dist.exists() {
        std::fs::remove_dir_all(&dist)
            .with_context(|| format!("清理旧 dist 失败：{}", dist.display()))?;
    }
    std::fs::create_dir_all(&dist)?;

    // 1) 二进制
    let release = root.join("target/release");
    let suffix = std::env::consts::EXE_SUFFIX;
    for name in ["drill-locker", "drill-restorer"] {
        let file = format!("{name}{suffix}");
        let src = release.join(&file);
        if !src.is_file() {
            bail!("找不到编译产物 {}，请先运行 cargo run -p xtask -- build", src.display());
        }
        let dst = dist.join(&file);
        std::fs::copy(&src, &dst)
            .with_context(|| format!("复制失败：{}", dst.display()))?;
        println!("    {file}");
    }

    // 2) 网页
    let index = dist.join("index.html");
    std::fs::write(&index, drill_core::template::index_html(&cfg))
        .with_context(|| format!("写入失败：{}", index.display()))?;
    let download = dist.join("download.html");
    std::fs::write(&download, drill_core::template::download_html(&cfg))
        .with_context(|| format!("写入失败：{}", download.display()))?;
    println!("    index.html");
    println!("    download.html");

    // 3) 壁纸（同时方便预览效果）
    let wp = dist.join("wallpaper.png");
    drill_core::wallpaper::render(&cfg, &wp)
        .with_context(|| format!("生成壁纸失败：{}", wp.display()))?;
    println!("    wallpaper.png");

    // 4) 启动脚本
    write_launchers(&dist)?;

    // 5) 演练须知
    let notice = dist.join("演练说明.txt");
    std::fs::write(&notice, notice_text(&cfg))
        .with_context(|| format!("写入失败：{}", notice.display()))?;
    println!("    演练说明.txt");

    println!();
    println!("打包完成。单位：{}（演练编号 {}）", cfg.organization.name, cfg.organization.drill_code);
    println!("把整个 {} 目录拷贝到演练机器上即可使用。", dist.display());
    println!();
    println!("演练：打开 index.html 展示钓鱼页面 → 运行 drill-locker");
    println!("恢复：运行 drill-restorer --yes");

    Ok(())
}

/// 只有类 Unix 平台需要显式加可执行位；Windows 靠扩展名判断。
#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = std::fs::metadata(path)?.permissions();
    perm.set_mode(0o755);
    std::fs::set_permissions(path, perm)?;
    Ok(())
}

fn write_launchers(dist: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let start = dist.join("start-drill.command");
        std::fs::write(
            &start,
            "#!/bin/bash\n\
             # 演练启动脚本：双击本文件即可执行演练\n\
             cd \"$(dirname \"$0\")\" || exit 1\n\
             ./drill-locker\n\
             echo\n\
             read -n 1 -s -r -p \"按任意键关闭此窗口…\"\n",
        )?;
        make_executable(&start)?;

        let restore = dist.join("restore.command");
        std::fs::write(
            &restore,
            "#!/bin/bash\n\
             # 一键恢复：双击本文件即可还原全部文件名与桌面壁纸\n\
             cd \"$(dirname \"$0\")\" || exit 1\n\
             ./drill-restorer --yes\n\
             echo\n\
             read -n 1 -s -r -p \"按任意键关闭此窗口…\"\n",
        )?;
        make_executable(&restore)?;
    }

    #[cfg(windows)]
    {
        let start = dist.join("start-drill.bat");
        std::fs::write(
            &start,
            "@echo off\r\n\
             chcp 65001 >nul\r\n\
             rem 演练启动脚本：双击本文件即可执行演练\r\n\
             cd /d \"%~dp0\"\r\n\
             drill-locker.exe\r\n\
             echo.\r\n\
             pause\r\n",
        )?;

        let restore = dist.join("restore.bat");
        std::fs::write(
            &restore,
            "@echo off\r\n\
             chcp 65001 >nul\r\n\
             rem 一键恢复：双击本文件即可还原全部文件名与桌面壁纸\r\n\
             cd /d \"%~dp0\"\r\n\
             drill-restorer.exe --yes\r\n\
             echo.\r\n\
             pause\r\n",
        )?;
    }

    println!("    start-drill / restore 启动脚本");
    Ok(())
}

fn notice_text(cfg: &drill_core::Config) -> String {
    let mut s = String::new();
    s.push_str("==================================================\n");
    s.push_str(&format!(" {} 勒索病毒应急演练 · 演示套件\n", cfg.organization.name));
    s.push_str(&format!(" 演练编号：{}\n", cfg.organization.drill_code));
    s.push_str("==================================================\n\n");

    s.push_str("【重要声明】\n");
    s.push_str("本套件是应急演练的演示道具，不是真实的勒索软件。它只做三件事：\n");
    s.push_str("  1. 给文件名追加后缀（不修改、不删除、不加密任何文件内容）；\n");
    s.push_str("  2. 把桌面壁纸换成黑底红字的提示图；\n");
    s.push_str("  3. 弹出一个仿冒的勒索窗口（不联网、不驻留）。\n");
    s.push_str("所有改动都是可逆的，请勿用于演练以外的任何用途。\n\n");

    s.push_str("【演练流程】\n");
    s.push_str("  1. 把本目录放在演练用的专用文件夹中（不要放在家目录或系统目录下）；\n");
    s.push_str("  2. 打开 index.html，向参演人员展示钓鱼下载页面；\n");
    s.push_str("  3. 运行 drill-locker（macOS 双击 start-drill.command，Windows 双击 start-drill.bat）；\n");
    s.push_str("  4. 观察参演人员的应急响应过程。\n\n");

    s.push_str("【恢复流程】\n");
    s.push_str("  macOS  ：双击 restore.command\n");
    s.push_str("  Windows：双击 restore.bat\n");
    s.push_str("  或手动执行：drill-restorer --yes\n\n");
    s.push_str("  想先确认会恢复什么，可以加 --dry-run 预览：\n");
    s.push_str("      drill-restorer --dry-run\n\n");

    s.push_str("【注意事项】\n");
    s.push_str("  - macOS 首次更换壁纸时，系统会询问是否允许控制「系统事件」，必须点「好」；\n");
    s.push_str("    若误点拒绝，请到『系统设置 → 隐私与安全性 → 自动化』中重新允许。\n");
    s.push_str("  - Windows 上安全软件可能拦截本程序，请在演练机器上临时放行。\n");
    s.push_str("  - 演练结束后务必运行恢复工具，确认文件名与壁纸均已还原。\n");
    s.push_str("  - 锁定记录保存在演练目录下的 manifest.json 中，删除它会导致无法精确还原\n");
    s.push_str("    （届时恢复工具仍可按后缀剥离方式还原文件名，但壁纸需手动设置）。\n");
    s
}

/// 造一批测试样本，用于在不碰真实数据的前提下试跑整套流程。
fn sandbox(dir: &Path) -> Result<()> {
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    std::fs::create_dir_all(dir)?;

    std::fs::write(dir.join("季度财务报表.xlsx"), b"demo spreadsheet")?;
    std::fs::write(dir.join("会议纪要.docx"), b"demo document")?;
    std::fs::write(dir.join("readme.txt"), b"demo text")?;
    std::fs::write(dir.join("photo.jpg"), vec![0u8; 4096])?;

    let sub = dir.join("子目录").join("更深一层");
    std::fs::create_dir_all(&sub)?;
    std::fs::write(sub.join("客户资料.pdf"), b"demo pdf")?;
    std::fs::write(sub.join("备份数据.bin"), vec![1u8; 8192])?;

    let excluded = dir.join("target");
    std::fs::create_dir_all(&excluded)?;
    std::fs::write(excluded.join("should_be_skipped.txt"), b"excluded")?;

    println!("已生成测试样本：{}", dir.display());
    println!("可以这样试跑：");
    println!("    ./target/release/drill-locker --root {} --dry-run", dir.display());
    println!("    ./target/release/drill-locker --root {}", dir.display());
    println!("    ./target/release/drill-restorer --root {} --yes", dir.display());

    Ok(())
}

fn wallpaper_only(out: &Path) -> Result<()> {
    let cfg = drill_core::config::load()?;
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    drill_core::wallpaper::render(&cfg, out)?;
    println!("壁纸已生成：{}", out.display());
    Ok(())
}
