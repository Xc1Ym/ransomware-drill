//! `drill-restorer` —— 演练恢复工具。
//!
//! 把 `drill-locker` 做过的两件事还原回去：
//!   1. 去掉文件名的伪加密后缀；
//!   2. 把桌面壁纸设回演练前的图片。
//!
//! 设计上刻意做得**宽容**：找不到 manifest 也能靠后缀剥离还原，
//! 目标文件已存在时宁可跳过也不覆盖。

use anyhow::{bail, Context, Result};
use drill_core::{config, restore};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

const HELP: &str = r#"drill-restorer —— 勒索病毒应急演练恢复工具

用法：
    drill-restorer [选项]

选项：
    -r, --root <目录>   指定要恢复的目录（默认为本程序所在目录）
    -n, --dry-run       只预览将要恢复的内容，不做任何改动
    -y, --yes           跳过确认提示，直接执行
    -h, --help          显示本帮助
    -V, --version       显示版本号

说明：
    恢复会做两件事：还原文件名的伪加密后缀、把桌面壁纸设回演练前的图片。
    优先使用锁定目录中的 manifest.json 精确还原；若该文件丢失，
    则退化为「去掉所有带该后缀的文件名」，同样可以恢复。

示例：
    drill-restorer --dry-run          # 先看看会恢复什么
    drill-restorer --yes              # 直接恢复
    drill-restorer --root ~/演练演示   # 指定目录
"#;

struct Args {
    root: Option<PathBuf>,
    dry_run: bool,
    yes: bool,
}

fn parse_args() -> Result<Option<Args>> {
    let mut args = Args {
        root: None,
        dry_run: false,
        yes: false,
    };

    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                println!("{HELP}");
                return Ok(None);
            }
            "-V" | "--version" => {
                println!("drill-restorer {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            "-n" | "--dry-run" => args.dry_run = true,
            "-y" | "--yes" => args.yes = true,
            "-r" | "--root" => {
                let v = it.next().context("--root 后面需要跟一个目录路径")?;
                args.root = Some(PathBuf::from(v));
            }
            other if other.starts_with("--root=") => {
                args.root = Some(PathBuf::from(&other["--root=".len()..]));
            }
            other => bail!("无法识别的参数：{other}\n运行 drill-restorer --help 查看用法。"),
        }
    }

    Ok(Some(args))
}

/// 默认演练目录：本程序所在目录。
///
/// 用程序自身位置而不是「当前工作目录」，因为双击运行时 cwd 在不同平台上
/// 可能是 `/` 或 `C:\Windows\System32`，与演练目录毫无关系。
fn default_root() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("无法定位本程序自身的位置")?;
    let dir = exe
        .parent()
        .context("无法确定本程序所在目录")?
        .to_path_buf();
    Ok(dir)
}

fn main() {
    if let Err(e) = run() {
        eprintln!("\n[错误] {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let Some(args) = parse_args()? else {
        return Ok(());
    };

    let cfg = config::load()?;

    let root = match args.root {
        Some(r) => r,
        None => default_root()?,
    };
    let root = root
        .canonicalize()
        .with_context(|| format!("演练目录不存在或无法访问：{}", root.display()))?;

    println!("==================================================");
    println!(" {} 应急演练 · 恢复工具", cfg.organization.name);
    println!(" 演练编号：{}", cfg.organization.drill_code);
    println!("==================================================");
    println!("待恢复目录：{}", root.display());
    println!("伪加密后缀：{}", cfg.lock.extension);

    if args.dry_run {
        println!("模式：预览（不会修改任何内容）");
    }
    println!();

    // 先预览一次，用于确认与展示。
    let preview = restore::restore_dir(&root, &cfg, true)?;

    if preview.restored == 0 && preview.missing.is_empty() {
        println!("没有发现需要恢复的文件。");
        println!("可能演练尚未执行，或该目录已经恢复过了。");
        // 即便没有文件要恢复，壁纸也可能还停在演练状态。
        if let Some(w) = preview.wallpaper_restored.clone() {
            println!("\n检测到需要还原的壁纸：{}", w.display());
            if !args.dry_run {
                restore::restore_dir(&root, &cfg, false)?;
                println!("壁纸已还原。");
            }
        }
        return Ok(());
    }

    println!("将恢复 {} 个文件。", preview.restored);
    if let Some(w) = &preview.wallpaper_restored {
        println!("将把桌面壁纸还原为：{}", w.display());
    } else if preview.used_manifest {
        println!("未记录演练前的壁纸路径，跳过壁纸还原。");
    } else {
        println!("未找到 manifest.json，壁纸不会自动还原（可手动设置）。");
    }
    println!();

    if args.dry_run {
        println!("预览完成。去掉 --dry-run 即可实际执行恢复。");
        return Ok(());
    }

    if !args.yes {
        if !std::io::stdin().is_terminal() {
            bail!("当前环境无法交互确认，请在命令后加上 --yes 直接执行。");
        }
        print!("确认执行恢复？输入 y 回车继续：");
        use std::io::Write;
        std::io::stdout().flush().ok();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        if !matches!(line.trim(), "y" | "Y" | "yes") {
            println!("已取消，未做任何改动。");
            return Ok(());
        }
    }

    let report = restore::restore_dir(&root, &cfg, false)?;

    println!();
    println!("---------------- 恢复结果 ----------------");
    println!("已还原文件：{} 个", report.restored);
    if !report.missing.is_empty() {
        println!("manifest 中登记但未找到：{} 个", report.missing.len());
        for p in report.missing.iter().take(10) {
            println!("    {}", p.display());
        }
        if report.missing.len() > 10 {
            println!("    …（其余 {} 个已省略）", report.missing.len() - 10);
        }
    }
    if !report.failed.is_empty() {
        println!("跳过/失败：{} 个", report.failed.len());
        for (p, why) in report.failed.iter().take(10) {
            println!("    {} —— {}", p.display(), why);
        }
    }
    match (&report.wallpaper_restored, &report.wallpaper_error) {
        (Some(w), _) => println!("桌面壁纸已还原为：{}", w.display()),
        (None, Some(e)) => println!("壁纸还原失败：{e}"),
        (None, None) => println!("壁纸未做改动。"),
    }
    if !report.used_manifest {
        println!("提示：本次未找到 manifest.json，是按后缀剥离方式恢复的。");
    }
    println!("------------------------------------------");

    if report.failed.is_empty() {
        println!("\n恢复完成。");
    } else {
        println!("\n恢复完成，但有 {} 项被跳过，请查看上方列表。", report.failed.len());
    }

    Ok(())
}

#[allow(dead_code)]
fn unused(_: &Path) {}
