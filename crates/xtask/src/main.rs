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
    package      完整打包：编译（本机 + 交叉编译 Windows）+ 渲染网页
                 + 生成壁纸 + 组装 dist/
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
    build_target(None)
}

/// 把 llvm-mingw 接进 cargo 的环境。
///
/// Windows arm64 用的 `aarch64-pc-windows-gnullvm` 目标需要 llvm-mingw 提供链接器，
/// 而它的安装位置因人而异，所以不写死在 .cargo/config.toml 里，
/// 改为读取 `LLVM_MINGW_BIN` 环境变量（指向 llvm-mingw 的根目录或 bin 目录）。
fn apply_llvm_mingw_env(cmd: &mut Command) {
    let Ok(root) = std::env::var("LLVM_MINGW_BIN") else {
        return;
    };
    let root = PathBuf::from(root);
    let bin = if root.join("bin").is_dir() {
        root.join("bin")
    } else {
        root
    };

    // 让 cargo 能在 PATH 里找到 aarch64-w64-mingw32-clang
    if let Some(path) = std::env::var_os("PATH") {
        let mut dirs = vec![bin.clone()];
        dirs.extend(std::env::split_paths(&path));
        if let Ok(joined) = std::env::join_paths(dirs) {
            cmd.env("PATH", joined);
        }
    }
    cmd.env(
        "CARGO_TARGET_AARCH64_PC_WINDOWS_GNULLVM_LINKER",
        bin.join("aarch64-w64-mingw32-clang"),
    );
}

/// 编译 release 版本；`target` 为 `None` 时编译本机版本。
fn build_target(target: Option<&str>) -> Result<()> {
    match target {
        Some(t) => println!("==> 编译 release 版本（交叉编译到 {t}）…"),
        None => println!("==> 编译 release 版本（本机）…"),
    }

    let mut cmd = Command::new(env!("CARGO"));
    cmd.args(["build", "--release"]);
    if let Some(t) = target {
        cmd.args(["--target", t]);
        // 只有 Windows arm64 这一步需要 llvm-mingw。若把它加进全局 PATH，
        // 会连带影响 x86_64-pc-windows-gnu 的链接器解析——llvm-mingw 里
        // 也有个同名 gcc wrapper，但缺少 libgcc，反而导致 x64 目标链接失败。
        if t.contains("gnullvm") {
            apply_llvm_mingw_env(&mut cmd);
        }
    }

    let status = cmd
        .current_dir(workspace_root())
        .status()
        .context("执行 cargo build 失败，请确认 cargo 在 PATH 中")?;

    if !status.success() {
        match target {
            Some(t) => bail!("cargo build --release --target {t} 失败"),
            None => bail!("cargo build --release 失败"),
        }
    }
    Ok(())
}

/// release 产物所在目录。
fn release_dir(target: Option<&str>) -> PathBuf {
    let root = workspace_root().join("target");
    match target {
        None => root.join("release"),
        Some(t) => root.join(t).join("release"),
    }
}

/// 一个构建目标。
struct BuildTarget {
    /// cargo 的 target 三元组；`None` 表示本机。
    triple: Option<&'static str>,
    /// 产物在 `dist/` 中的平台后缀，例如 `-darwin-arm64`。
    artifact_suffix: String,
    /// 可执行文件扩展名。
    exe_suffix: &'static str,
    label: &'static str,
    /// 需要什么才能构建；空串表示本机，无需额外工具链。
    requires: &'static str,
}

/// 本机平台在产物名里的后缀，与旧版 Release 的命名风格保持一致
/// （darwin-arm64 / windows-amd64 / linux-amd64 ...）。
fn host_artifact_suffix() -> String {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "amd64",
        other => other,
    };
    format!("-{os}-{arch}")
}

/// 打包时要构建的目标清单。
///
/// 除了本机，还会尝试交叉编译其余平台，让 `dist/` 成为一份「拿到哪台机器都能用」
/// 的完整包。某个平台缺工具链时只跳过该平台并给出安装提示，不影响整体打包。
fn build_targets() -> Vec<BuildTarget> {
    let host = BuildTarget {
        triple: None,
        artifact_suffix: host_artifact_suffix(),
        exe_suffix: std::env::consts::EXE_SUFFIX,
        label: "本机",
        requires: "",
    };
    let host_suffix = host.artifact_suffix.clone();

    let mut v = vec![host];

    let cross = [
        BuildTarget {
            triple: Some("x86_64-pc-windows-gnu"),
            artifact_suffix: "-windows-amd64".to_string(),
            exe_suffix: ".exe",
            label: "Windows amd64",
            requires: "mingw-w64（macOS: brew install mingw-w64）",
        },
        BuildTarget {
            triple: Some("aarch64-pc-windows-gnullvm"),
            artifact_suffix: "-windows-arm64".to_string(),
            exe_suffix: ".exe",
            label: "Windows arm64",
            requires: "llvm-mingw（https://github.com/mstorsjo/llvm-mingw）",
        },
        BuildTarget {
            triple: Some("x86_64-unknown-linux-gnu"),
            artifact_suffix: "-linux-amd64".to_string(),
            exe_suffix: "",
            label: "Linux amd64",
            requires: "Linux 交叉工具链（brew tap messense/macos-cross-toolchains）",
        },
        BuildTarget {
            triple: Some("aarch64-unknown-linux-gnu"),
            artifact_suffix: "-linux-arm64".to_string(),
            exe_suffix: "",
            label: "Linux arm64",
            requires: "Linux 交叉工具链（brew tap messense/macos-cross-toolchains）",
        },
    ];

    for t in cross {
        // 本机平台已经作为 host 构建过了，不必再交叉编译一遍
        if t.artifact_suffix == host_suffix {
            continue;
        }
        v.push(t);
    }

    v
}

/// 组装可直接拷贝到演练机器使用的 `dist/` 目录。
fn package() -> Result<()> {
    let root = workspace_root();
    let cfg = drill_core::config::load()?;
    let dist = root.join("dist");

    // 先编译：本机必须成功；其余平台失败只警告并跳过，不影响整体打包。
    let mut built: Vec<BuildTarget> = Vec::new();
    let mut skipped: Vec<(String, String)> = Vec::new();

    for t in build_targets() {
        match build_target(t.triple) {
            Ok(()) => built.push(t),
            Err(e) => {
                if t.triple.is_none() {
                    return Err(e);
                }
                println!();
                println!("[警告] 跳过 {}：{e}", t.label);
                println!("        需要：{}", t.requires);
                println!();
                skipped.push((t.label.to_string(), t.requires.to_string()));
            }
        }
    }

    println!("==> 组装 dist/ …");
    if dist.exists() {
        std::fs::remove_dir_all(&dist)
            .with_context(|| format!("清理旧 dist 失败：{}", dist.display()))?;
    }
    std::fs::create_dir_all(&dist)?;

    // 1) 二进制：每个成功编译的平台各放一份，文件名带平台后缀便于分发时区分
    for t in &built {
        let release = release_dir(t.triple);
        for name in ["drill-locker", "drill-restorer"] {
            let src = release.join(format!("{name}{}", t.exe_suffix));
            if !src.is_file() {
                bail!("找不到编译产物 {}", src.display());
            }
            let file = format!("{name}{}{}", t.artifact_suffix, t.exe_suffix);
            let dst = dist.join(&file);
            std::fs::copy(&src, &dst)
                .with_context(|| format!("复制失败：{}", dst.display()))?;
            println!("    {file}");
        }
    }

    // 2) 网页
    for (name, html) in [
        ("index.html", drill_core::template::index_html(&cfg)),
        ("download.html", drill_core::template::download_html(&cfg)),
        ("search.html", drill_core::template::search_html(&cfg)),
    ] {
        let path = dist.join(name);
        std::fs::write(&path, html).with_context(|| format!("写入失败：{}", path.display()))?;
        println!("    {name}");
    }

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

/// 生成三个平台的启动脚本。
///
/// 刻意不按构建主机裁剪：打包出的 `dist/` 要能同时给 macOS、Windows 和 Linux 用，
/// 所以在哪个平台打包都生成全套脚本。脚本内部会自行判断机器架构，
/// 挑选对应的那份二进制。
fn write_launchers(dist: &Path) -> Result<()> {
    // ---------- macOS ----------
    {
        let start = dist.join("start-drill.command");
        std::fs::write(
            &start,
            "#!/bin/bash\n\
             # 演练启动脚本：双击本文件即可执行演练\n\
             cd \"$(dirname \"$0\")\" || exit 1\n\
             case \"$(uname -m)\" in\n\
             \x20 arm64) BIN=./drill-locker-darwin-arm64 ;;\n\
             \x20 *)     BIN=./drill-locker-darwin-amd64 ;;\n\
             esac\n\
             if [ ! -f \"$BIN\" ]; then BIN=$(ls ./drill-locker-darwin-* 2>/dev/null | head -1); fi\n\
             if [ -z \"$BIN\" ] || [ ! -f \"$BIN\" ]; then\n\
             \x20 echo \"未找到适用于本机的演练程序（drill-locker-darwin-*）。\"\n\
             \x20 read -n 1 -s -r -p \"按任意键关闭此窗口…\"\n\
             \x20 exit 1\n\
             fi\n\
             chmod +x \"$BIN\" 2>/dev/null\n\
             \"$BIN\"\n\
             echo\n\
             read -n 1 -s -r -p \"按任意键关闭此窗口…\"\n",
        )?;
        #[cfg(unix)]
        make_executable(&start)?;

        let restore = dist.join("restore.command");
        std::fs::write(
            &restore,
            "#!/bin/bash\n\
             # 一键恢复：双击本文件即可还原全部文件名与桌面壁纸\n\
             cd \"$(dirname \"$0\")\" || exit 1\n\
             case \"$(uname -m)\" in\n\
             \x20 arm64) BIN=./drill-restorer-darwin-arm64 ;;\n\
             \x20 *)     BIN=./drill-restorer-darwin-amd64 ;;\n\
             esac\n\
             if [ ! -f \"$BIN\" ]; then BIN=$(ls ./drill-restorer-darwin-* 2>/dev/null | head -1); fi\n\
             if [ -z \"$BIN\" ] || [ ! -f \"$BIN\" ]; then\n\
             \x20 echo \"未找到适用于本机的恢复工具（drill-restorer-darwin-*）。\"\n\
             \x20 read -n 1 -s -r -p \"按任意键关闭此窗口…\"\n\
             \x20 exit 1\n\
             fi\n\
             chmod +x \"$BIN\" 2>/dev/null\n\
             \"$BIN\" --yes\n\
             echo\n\
             read -n 1 -s -r -p \"按任意键关闭此窗口…\"\n",
        )?;
        #[cfg(unix)]
        make_executable(&restore)?;
    }

    // ---------- Windows ----------
    {
        let start = dist.join("start-drill.bat");
        std::fs::write(
            &start,
            "@echo off\r\n\
             chcp 65001 >nul\r\n\
             rem 演练启动脚本：双击本文件即可执行演练\r\n\
             cd /d \"%~dp0\"\r\n\
             set \"BIN=drill-locker-windows-amd64.exe\"\r\n\
             if /i \"%PROCESSOR_ARCHITECTURE%\"==\"ARM64\" set \"BIN=drill-locker-windows-arm64.exe\"\r\n\
             if not exist \"%BIN%\" set \"BIN=drill-locker-windows-amd64.exe\"\r\n\
             if not exist \"%BIN%\" (\r\n\
             \x20 echo 未找到适用于本机的演练程序（drill-locker-windows-*.exe）。\r\n\
             \x20 pause\r\n\
             \x20 exit /b 1\r\n\
             )\r\n\
             \"%BIN%\"\r\n\
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
             set \"BIN=drill-restorer-windows-amd64.exe\"\r\n\
             if /i \"%PROCESSOR_ARCHITECTURE%\"==\"ARM64\" set \"BIN=drill-restorer-windows-arm64.exe\"\r\n\
             if not exist \"%BIN%\" set \"BIN=drill-restorer-windows-amd64.exe\"\r\n\
             if not exist \"%BIN%\" (\r\n\
             \x20 echo 未找到适用于本机的恢复工具（drill-restorer-windows-*.exe）。\r\n\
             \x20 pause\r\n\
             \x20 exit /b 1\r\n\
             )\r\n\
             \"%BIN%\" --yes\r\n\
             echo.\r\n\
             pause\r\n",
        )?;
    }

    // ---------- Linux ----------
    {
        for (script, tool) in [
            ("start-drill.sh", "drill-locker"),
            ("restore.sh", "drill-restorer"),
        ] {
            let path = dist.join(script);
            let tail = if tool == "drill-restorer" { " --yes" } else { "" };
            std::fs::write(
                &path,
                format!(
                    "#!/bin/sh\n\
                     # {title}：在终端里执行本脚本即可\n\
                     cd \"$(dirname \"$0\")\" || exit 1\n\
                     case \"$(uname -m)\" in\n\
                     \x20 aarch64|arm64) BIN=./{tool}-linux-arm64 ;;\n\
                     \x20 *)              BIN=./{tool}-linux-amd64 ;;\n\
                     esac\n\
                     if [ ! -f \"$BIN\" ]; then\n\
                     \x20 echo \"未找到适用于本机的程序（{tool}-linux-*）。\"\n\
                     \x20 exit 1\n\
                     fi\n\
                     chmod +x \"$BIN\" 2>/dev/null\n\
                     \"$BIN\"{tail}\n",
                    title = if tool == "drill-restorer" {
                        "一键恢复"
                    } else {
                        "演练启动脚本"
                    },
                ),
            )?;
            #[cfg(unix)]
            make_executable(&path)?;
        }
    }

    println!("    start-drill / restore 启动脚本（macOS .command、Windows .bat、Linux .sh）");
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

    s.push_str("【本目录内容】\n");
    s.push_str("  drill-locker-<平台>       演练程序（按平台区分）\n");
    s.push_str("  drill-restorer-<平台>     恢复工具\n");
    s.push_str("  平台后缀：darwin-arm64 / windows-amd64 / windows-arm64\n");
    s.push_str("            linux-amd64 / linux-arm64\n");
    s.push_str("  start-drill.command       macOS：双击启动演练\n");
    s.push_str("  start-drill.bat           Windows：双击启动演练\n");
    s.push_str("  start-drill.sh            Linux：在终端执行\n");
    s.push_str("  restore.command / .bat / .sh   对应平台的一键恢复\n");
    s.push_str("  index.html / search.html / download.html  仿冒的浏览器与搜索页面\n");
    s.push_str("  wallpaper.png             演练用的桌面壁纸\n");
    s.push_str("  manifest.json             锁定记录（演练后自动生成，请勿删除）\n\n");

    s.push_str("【演练流程】\n");
    s.push_str("  1. 把本目录放在演练用的专用文件夹中（不要放在家目录或系统目录下）；\n");
    s.push_str("  2. 打开 index.html，在搜索框里输入关键词（例如「chrome 下载」）并回车，\n");
    s.push_str("     进入仿搜索结果页，点击置顶的那条「广告」结果，进入下载页；\n");
    s.push_str("  3. 在下载页点「立即下载」；或直接运行 drill-locker\n");
    s.push_str("     （macOS 双击 start-drill.command，Windows 双击 start-drill.bat）；\n");
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
