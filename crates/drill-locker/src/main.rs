//! `drill-locker` —— 应急演练的「攻击端」演示程序。
//!
//! # 它做什么
//!
//! 1. 把所在目录下的文件**改名**（追加伪加密后缀）；
//! 2. 把桌面壁纸换成黑底红字的中文提示图；
//! 3. 弹出一个仿 Wana Decrypt0r 的窗口。
//!
//! # 它不做什么
//!
//! - **不读取、不修改、不删除任何文件内容**——全部动作只有一次 `fs::rename`；
//! - **不加密**：代码里没有任何密码学运算；
//! - **不联网**：没有任何网络代码，弹窗上的邮箱与比特币地址均为虚构；
//! - **不驻留**：窗口关闭即进程退出，锁的状态由文件名维持，不依赖后台进程；
//! - **不碰系统目录**：见 `drill_core::safety` 的黑名单。
//!
//! 演练结束后运行同目录下的 `drill-restorer` 即可完全还原。

mod popup;
mod theme;

use anyhow::{bail, Context, Result};
use chrono::Local;
use drill_core::{config, lock, platform, wallpaper, Config};
use std::path::PathBuf;

const HELP: &str = r#"drill-locker —— 勒索病毒应急演练演示程序（攻击端）

用法：
    drill-locker [选项]

选项：
    -r, --root <目录>   指定演练目录（默认为本程序所在目录）
    -n, --dry-run       只预览将要改名的文件，不做任何改动（壁纸和弹窗也不会触发）
        --no-wallpaper  不更换桌面壁纸
        --no-gui        只执行改名，不弹出勒索窗口（便于自动化验证）
        --screenshot <文件>
                        截取弹窗自身并保存为 PNG 后退出（用于生成文档配图）。
                        只截取本程序自己的窗口，不会捕获桌面上其它内容。
    -h, --help          显示本帮助
    -V, --version       显示版本号

安全说明：
    本程序只对文件名追加后缀，不会修改或删除任何文件内容，也不会联网。
    演练结束后请运行同目录下的 drill-restorer 还原文件名与桌面壁纸。

示例：
    drill-locker --dry-run             # 先看看会影响哪些文件
    drill-locker                       # 执行演练（改名 + 换壁纸 + 弹窗）
    drill-locker --root ~/演练演示      # 指定目录
"#;

struct Args {
    root: Option<PathBuf>,
    dry_run: bool,
    no_wallpaper: bool,
    no_gui: bool,
    screenshot: Option<PathBuf>,
}

fn parse_args() -> Result<Option<Args>> {
    let mut args = Args {
        root: None,
        dry_run: false,
        no_wallpaper: false,
        no_gui: false,
        screenshot: None,
    };

    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                println!("{HELP}");
                return Ok(None);
            }
            "-V" | "--version" => {
                println!("drill-locker {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            "-n" | "--dry-run" => args.dry_run = true,
            "--no-wallpaper" => args.no_wallpaper = true,
            "--no-gui" => args.no_gui = true,
            "-r" | "--root" => {
                let v = it.next().context("--root 后面需要跟一个目录路径")?;
                args.root = Some(PathBuf::from(v));
            }
            "--screenshot" => {
                let v = it.next().context("--screenshot 后面需要跟一个文件路径")?;
                args.screenshot = Some(PathBuf::from(v));
            }
            other if other.starts_with("--root=") => {
                args.root = Some(PathBuf::from(&other["--root=".len()..]));
            }
            other if other.starts_with("--screenshot=") => {
                args.screenshot = Some(PathBuf::from(&other["--screenshot=".len()..]));
            }
            other => bail!("无法识别的参数：{other}\n运行 drill-locker --help 查看用法。"),
        }
    }

    Ok(Some(args))
}

/// 默认演练目录：本程序所在目录。
///
/// 不用「当前工作目录」——双击运行时它在 macOS 与 Windows 上都可能是
/// 与演练无关的位置（例如 `/` 或 `C:\Windows\System32`）。
fn default_root() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("无法定位本程序自身的位置")?;
    Ok(exe
        .parent()
        .context("无法确定本程序所在目录")?
        .to_path_buf())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("\n[已中止] {e:#}");
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
    println!(" {} 应急演练 · 演示程序", cfg.organization.name);
    println!(" 演练编号：{}", cfg.organization.drill_code);
    println!("==================================================");
    println!("演练目录：{}", root.display());
    println!("伪加密后缀：{}", cfg.lock.extension);
    println!("说明：仅重命名文件，不会修改或删除任何文件内容。");
    println!();

    // 1) 锁定前先记下当前壁纸，供恢复时还原。
    let original_wallpaper = if args.no_wallpaper {
        None
    } else {
        match platform::get_wallpaper() {
            Ok(w) => w,
            Err(e) => {
                println!("[提示] 未能读取当前壁纸（{e}），恢复时将无法自动还原壁纸。");
                None
            }
        }
    };

    // 2) 锁定文件（只改名）。
    let report = lock::lock_dir(&root, &cfg, args.dry_run, original_wallpaper)?;

    if args.dry_run {
        let planned = drill_core::walk::collect_files(
            &root,
            cfg.lock.recursive,
            &cfg.effective_exclude(),
            &cfg.lock.extension,
        )?;
        println!("[预览] 共发现 {} 个文件将被重命名：", planned.len());
        for p in planned.iter().take(20) {
            println!("    {}  ->  {}{}", p.display(), p.display(), cfg.lock.extension);
        }
        if planned.len() > 20 {
            println!("    …（其余 {} 个已省略）", planned.len() - 20);
        }
        println!();
        println!("预览完成，未做任何改动（壁纸与弹窗也不会触发）。");
        return Ok(());
    }

    println!("[完成] 已锁定 {} 个文件（仅重命名，内容未改动）", report.locked);
    if report.skipped > 0 {
        println!("[提示] 跳过 {} 个（已锁定或已不存在）。", report.skipped);
    }
    if !report.failed.is_empty() {
        println!("[警告] {} 个文件改名失败：", report.failed.len());
        for (p, why) in report.failed.iter().take(5) {
            println!("    {} —— {}", p.display(), why);
        }
    }
    if report.notes_written > 0 {
        println!(
            "[完成] 已在 {} 个目录投放勒索信：{}",
            report.notes_written, cfg.ransom_note.filename
        );
    }
    if let Some(m) = &report.manifest_path {
        println!("[完成] 恢复清单已写入：{}", m.display());
    }

    // 3) 换壁纸
    if args.no_wallpaper {
        println!("[跳过] 按参数要求不更换桌面壁纸。");
    } else {
        match wallpaper::render_to_temp(&cfg) {
            Ok(path) => match platform::set_wallpaper(&path) {
                Ok(()) => println!("[完成] 桌面壁纸已更换：{}", path.display()),
                Err(e) => println!("[警告] 更换壁纸失败：{e}"),
            },
            Err(e) => println!("[警告] 生成壁纸失败：{e}"),
        }
    }

    // 4) 弹窗
    if args.no_gui {
        println!("[跳过] 按参数要求不弹出窗口。");
    } else {
        println!("[完成] 弹出勒索窗口…");
        run_popup(cfg, args.screenshot)?;
        println!();
        println!("窗口已关闭。文件仍处于锁定状态，");
        println!("请运行同目录下的 drill-restorer 完成还原。");
    }

    Ok(())
}

/// 加载中文字体到 egui。
///
/// 不内嵌字体文件，运行时从系统读取，避免让演练程序体积异常膨胀。
fn setup_fonts(ctx: &egui::Context, cfg: &Config) -> Result<()> {
    let font = drill_core::fonts::load(cfg.font_override().as_deref())?;
    let mut data = egui::FontData::from_owned(font.data);
    data.index = font.index;

    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("cjk".to_owned(), std::sync::Arc::new(data));

    // 放在首位：中英文都用同一套黑体，观感才统一。
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "cjk".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("cjk".to_owned());

    ctx.set_fonts(fonts);
    Ok(())
}

fn run_popup(cfg: Config, screenshot: Option<PathBuf>) -> Result<()> {
    let title = cfg.popup.title.clone();
    let cfg_for_fonts = cfg.clone();
    let started = Local::now();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([theme::WINDOW_W, theme::WINDOW_H])
            .with_min_inner_size([theme::WINDOW_W, theme::WINDOW_H])
            .with_max_inner_size([theme::WINDOW_W + 400.0, theme::WINDOW_H + 400.0])
            .with_resizable(true)
            .with_decorations(false)
            .with_title(&title),
        ..Default::default()
    };

    eframe::run_native(
        &title,
        options,
        Box::new(move |cc| {
            setup_fonts(&cc.egui_ctx, &cfg_for_fonts)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.into() })?;
            let app: Box<dyn eframe::App> =
                Box::new(popup::PopupApp::new(cfg.clone(), started, screenshot));
            Ok(app)
        }),
    )
    .map_err(|e| anyhow::anyhow!("启动弹窗失败：{e}"))?;

    Ok(())
}
