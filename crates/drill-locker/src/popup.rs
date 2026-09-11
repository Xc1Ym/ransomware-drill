//! 演练弹窗：按参考图（Wana Decrypt0r 2.0）复刻的界面。
//!
//! 整个窗口是**纯本地渲染**的，没有任何网络请求。两个按钮点击后只会弹出
//! 一段本地写死的模拟提示，不会连接任何服务器，也不存在任何真实的付款逻辑。

use crate::theme;
use anyhow::{Context as _, Result};
use chrono::{DateTime, Local};
use drill_core::Config;
use egui::{Align2, Color32, FontId, RichText, Sense, Stroke, Vec2};
use std::path::{Path, PathBuf};

/// 模拟提示框。
#[derive(Clone)]
struct Dialog {
    title: String,
    text: String,
}

pub struct PopupApp {
    cfg: Config,
    headline: String,
    body: String,
    /// 弹窗出现的时间，倒计时以此为基准。
    started: DateTime<Local>,
    dialog: Option<Dialog>,
    /// Copy 按钮的短暂反馈。
    copied_at: Option<std::time::Instant>,
    /// 自截图模式：渲染稳定后截取本窗口内容写入该路径，然后退出。
    /// 只截取本程序自己的窗口，不会捕获桌面上其它内容。
    screenshot_path: Option<PathBuf>,
    frame_count: u32,
}

impl PopupApp {
    pub fn new(cfg: Config, started: DateTime<Local>, screenshot_path: Option<PathBuf>) -> Self {
        let headline = cfg.expand(&cfg.popup.headline);
        let body = cfg.popup_body();
        Self {
            cfg,
            headline,
            body,
            started,
            dialog: None,
            copied_at: None,
            screenshot_path,
            frame_count: 0,
        }
    }

    /// 自截图：等界面渲染稳定后请求截图，收到后保存并关闭窗口。
    fn handle_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.screenshot_path.clone() else {
            return;
        };

        self.frame_count += 1;
        // 等若干帧，确保字体与布局都已稳定
        if self.frame_count == 20 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }

        let mut captured: Option<std::sync::Arc<egui::ColorImage>> = None;
        ctx.input(|i| {
            for ev in &i.events {
                if let egui::Event::Screenshot { image, .. } = ev {
                    captured = Some(image.clone());
                }
            }
        });

        if let Some(image) = captured {
            match save_color_image(&image, &path) {
                Ok(()) => println!("[完成] 弹窗截图已保存：{}", path.display()),
                Err(e) => eprintln!("[警告] 保存截图失败：{e:#}"),
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn pay_deadline(&self) -> DateTime<Local> {
        self.started + chrono::Duration::hours(self.cfg.popup.pay_raise_hours)
    }

    fn lost_deadline(&self) -> DateTime<Local> {
        self.started + chrono::Duration::hours(self.cfg.popup.files_lost_hours)
    }

    // ---------------------------------------------------------------- 标题栏

    fn title_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let rect = ui.max_rect();
        paint_v_gradient(
            ui.painter(),
            rect,
            theme::TITLEBAR_TOP,
            theme::TITLEBAR_BOTTOM,
        );

        // 拖动窗口
        let drag = ui.interact(rect, egui::Id::new("titlebar-drag"), Sense::drag());
        if drag.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        // 左上角的小图标
        let icon =
            egui::Rect::from_min_size(rect.left_top() + Vec2::new(7.0, 7.0), Vec2::splat(14.0));
        ui.painter().rect_filled(icon, 2, theme::LOCK_BG);
        ui.painter().rect_filled(
            egui::Rect::from_min_size(icon.min + Vec2::new(4.0, 6.0), Vec2::new(6.0, 6.0)),
            1,
            theme::LOCK_FG,
        );

        // 标题文字
        ui.painter().text(
            rect.left_center() + Vec2::new(28.0, 0.0),
            Align2::LEFT_CENTER,
            &self.cfg.popup.title,
            FontId::proportional(13.0),
            Color32::WHITE,
        );

        // 右侧窗口按钮：× □ —
        let btn_w = 28.0;
        for (i, label) in ["×", "□", "—"].iter().enumerate() {
            let r = egui::Rect::from_min_size(
                egui::pos2(
                    rect.right() - 3.0 - (i as f32 + 1.0) * btn_w,
                    rect.top() + 3.0,
                ),
                Vec2::new(btn_w - 2.0, rect.height() - 6.0),
            );
            let resp = ui.interact(r, egui::Id::new(("winbtn", i)), Sense::click());
            if resp.hovered() {
                ui.painter()
                    .rect_filled(r, 2, Color32::from_rgb(0x7A, 0x28, 0x60));
            }
            ui.painter().text(
                r.center(),
                Align2::CENTER_CENTER,
                *label,
                FontId::proportional(12.0),
                Color32::WHITE,
            );
            // 只有 × 有行为：关闭窗口。关闭后文件仍保持锁定状态，
            // 必须用恢复工具才能还原——这正是演练想传达的点。
            if i == 0 && resp.clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    // ---------------------------------------------------------------- 左栏

    fn left_panel(&mut self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        paint_v_gradient(
            ui.painter(),
            rect,
            theme::LEFT_PANEL_TOP,
            theme::LEFT_PANEL_BOTTOM,
        );

        ui.vertical(|ui| {
            self.lock_icon(ui);
            ui.add_space(6.0);
            self.countdown_box(ui, "Payment will be raised on", self.pay_deadline());
            ui.add_space(7.0);
            self.countdown_box(ui, "Your files will be lost on", self.lost_deadline());
            ui.add_space(10.0);

            let mut opened: Option<Dialog> = None;

            for link in ["About bitcoin", "How to buy bitcoins?"] {
                let resp = ui.add(
                    egui::Label::new(
                        RichText::new(link)
                            .size(11.5)
                            .color(theme::LINK_BLUE)
                            .underline(),
                    )
                    .sense(Sense::click()),
                );
                if resp.clicked() {
                    opened = Some(Dialog {
                        title: "About bitcoin".into(),
                        text: "比特币是一种去中心化的数字货币。\n\n\
                               本窗口为应急演练演示环境，不涉及任何真实交易，\
                               也不会访问互联网。"
                            .into(),
                    });
                }
            }

            ui.add_space(8.0);
            let resp = ui.add(
                egui::Label::new(
                    RichText::new("Contact Us")
                        .size(12.5)
                        .color(theme::LINK_BLUE)
                        .underline()
                        .strong(),
                )
                .sense(Sense::click()),
            );
            if resp.clicked() {
                opened = Some(Dialog {
                    title: "Contact Us".into(),
                    text: format!(
                        "您可以通过以下邮箱与我们联系：\n\n{}\n\n\
                         （演示环境：该邮箱为虚构地址，不会实际收发邮件。）",
                        self.cfg.popup.email
                    ),
                });
            }

            if let Some(d) = opened {
                self.dialog = Some(d);
            }
        });
    }

    fn lock_icon(&self, ui: &mut egui::Ui) {
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 76.0), Sense::hover());
        ui.painter().rect_filled(rect, 3, theme::LOCK_BG);

        let c = rect.center();
        // 锁梁：画一个圆环，下半部分随后会被锁体盖住，形成「门」形。
        ui.painter()
            .circle_stroke(c + Vec2::new(0.0, -14.0), 13.0, Stroke::new(6.0, theme::LOCK_FG));
        // 锁体
        let body = egui::Rect::from_center_size(c + Vec2::new(0.0, 8.0), Vec2::new(40.0, 32.0));
        ui.painter().rect_filled(body, 3, theme::LOCK_FG);
        // 钥匙孔
        ui.painter()
            .circle_filled(body.center() + Vec2::new(0.0, -4.0), 3.6, theme::LOCK_BG);
        ui.painter().rect_filled(
            egui::Rect::from_center_size(
                body.center() + Vec2::new(0.0, 5.0),
                Vec2::new(3.0, 12.0),
            ),
            1,
            theme::LOCK_BG,
        );
    }

    fn countdown_box(&self, ui: &mut egui::Ui, title: &str, deadline: DateTime<Local>) {
        egui::Frame::NONE
            .fill(theme::COUNTDOWN_BG)
            .stroke(Stroke::new(1.0, theme::COUNTDOWN_BORDER))
            .corner_radius(2)
            .inner_margin(egui::Margin::same(6))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    ui.label(
                        RichText::new(title)
                            .size(10.5)
                            .color(theme::TIMER_FG)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(deadline.format("%m/%d/%Y %H:%M:%S").to_string())
                            .size(10.5)
                            .color(theme::TEXT_DARK),
                    );
                    ui.label(
                        RichText::new("Time Left")
                            .size(10.5)
                            .color(theme::TIMER_FG)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(fmt_time_left(deadline))
                            .size(20.0)
                            .color(theme::TIMER_FG)
                            .strong(),
                    );
                });
            });
    }

    // ---------------------------------------------------------------- 正文区

    fn body_panel(&mut self, ui: &mut egui::Ui) {
        // 顶部红色标题条
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::hover());
        ui.painter().rect_filled(rect, 0, theme::HEADER_RED);
        ui.painter().text(
            rect.left_center() + Vec2::new(10.0, 0.0),
            Align2::LEFT_CENTER,
            &self.headline,
            FontId::proportional(14.5),
            Color32::WHITE,
        );

        // 右上角的语言下拉（纯装饰）
        let dd = egui::Rect::from_min_size(
            egui::pos2(rect.right() - 110.0, rect.top() + 5.0),
            Vec2::new(102.0, 18.0),
        );
        ui.painter().rect_filled(dd, 2, Color32::WHITE);
        ui.painter().text(
            dd.left_center() + Vec2::new(5.0, 0.0),
            Align2::LEFT_CENTER,
            "Chinese (simpl.)",
            FontId::proportional(10.0),
            Color32::BLACK,
        );
        // 下拉箭头同样自绘：几何图形比缺字形的 Unicode 符号更可靠
        let tip = dd.right_center() + Vec2::new(-8.0, 0.0);
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                tip + Vec2::new(-4.5, -2.5),
                tip + Vec2::new(4.5, -2.5),
                tip + Vec2::new(0.0, 3.0),
            ],
            Color32::BLACK,
            Stroke::NONE,
        ));

        ui.add_space(5.0);

        // 正文滚动区
        egui::Frame::NONE
            .fill(theme::BODY_BG)
            .stroke(Stroke::new(1.0, theme::BODY_BORDER))
            .inner_margin(egui::Margin::same(9))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_height(ui.available_height());
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        for (i, para) in self.body.split("\n\n").enumerate() {
                            if i > 0 {
                                ui.add_space(9.0);
                            }
                            let mut lines = para.lines().filter(|l| !l.trim().is_empty());
                            if let Some(head) = lines.next() {
                                ui.label(
                                    RichText::new(head)
                                        .size(13.5)
                                        .strong()
                                        .color(theme::TEXT_DARK),
                                );
                            }
                            for line in lines {
                                ui.label(RichText::new(line).size(12.5).color(theme::TEXT_DARK));
                            }
                        }
                        ui.add_space(6.0);
                    });
            });
    }

    // ---------------------------------------------------------------- 付款条

    fn pay_bar(&mut self, ui: &mut egui::Ui) {
        let mut copied = false;

        ui.horizontal_centered(|ui| {
            // 比特币标识：系统字体普遍缺少 U+20BF(₿) 字形，这里用
            // 字母 B 加两道竖线自己拼出比特币符号，避免渲染成豆腐块。
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
            let c = rect.center();
            ui.painter().circle_filled(c, 19.0, Color32::WHITE);
            ui.painter().circle_filled(c, 17.0, theme::BITCOIN_ORANGE);
            ui.painter().text(
                c + Vec2::new(0.0, 1.0),
                Align2::CENTER_CENTER,
                "B",
                FontId::proportional(22.0),
                Color32::WHITE,
            );
            for dy in [-11.0, 8.0] {
                ui.painter().rect_filled(
                    egui::Rect::from_center_size(c + Vec2::new(0.0, dy), Vec2::new(2.5, 7.0)),
                    0,
                    Color32::WHITE,
                );
            }

            ui.add_space(8.0);

            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                ui.label(
                    RichText::new(format!(
                        "Send {} worth of bitcoin to this address:",
                        self.cfg.popup.amount
                    ))
                    .size(11.5)
                    .color(Color32::WHITE)
                    .strong(),
                );

                ui.horizontal(|ui| {
                    // 地址框
                    egui::Frame::NONE
                        .fill(Color32::WHITE)
                        .corner_radius(2)
                        .inner_margin(egui::Margin::symmetric(7, 3))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(&self.cfg.popup.bitcoin)
                                    .size(12.5)
                                    .monospace()
                                    .color(Color32::BLACK),
                            );
                        });

                    let copied_recently = self
                        .copied_at
                        .map(|t| t.elapsed().as_secs() < 2)
                        .unwrap_or(false);
                    let label = if copied_recently { "Copied" } else { "Copy" };
                    if ui
                        .add_sized(
                            [62.0, 22.0],
                            egui::Button::new(RichText::new(label).size(11.5)),
                        )
                        .clicked()
                    {
                        copied = true;
                    }
                });
            });
        });

        if copied {
            ui.ctx().copy_text(self.cfg.popup.bitcoin.clone());
            self.copied_at = Some(std::time::Instant::now());
        }
    }

    // ---------------------------------------------------------------- 按钮条

    fn button_bar(&mut self, ui: &mut egui::Ui) {
        let mut clicked: Option<Dialog> = None;

        ui.horizontal_centered(|ui| {
            let total = ui.available_width();
            let w = (total - 16.0) / 2.0;

            let check = ui.add_sized(
                [w, 30.0],
                egui::Button::new(
                    RichText::new("Check Payment")
                        .size(12.5)
                        .color(theme::BUTTON_FG),
                )
                .fill(theme::BUTTON_BG),
            );
            if check.clicked() {
                clicked = Some(Dialog {
                    title: "Check Payment".into(),
                    text: format!(
                        "未收到您的付款。\n\n\
                         我们的服务器尚未检测到您支付的 {}。\n\
                         请确认您已向指定地址转账，并在 1-2 小时后重试。\n\n\
                         （演示环境：本程序不会联网，此处仅为模拟提示。）",
                        self.cfg.popup.amount
                    ),
                });
            }

            let decrypt = ui.add_sized(
                [w, 30.0],
                egui::Button::new(RichText::new("Decrypt").size(12.5).color(theme::BUTTON_FG))
                    .fill(theme::BUTTON_BG),
            );
            if decrypt.clicked() {
                clicked = Some(Dialog {
                    title: "Decrypt".into(),
                    text: "请先完成付款。\n\n\
                         付款成功后点击 Check Payment 验证，\
                         验证通过即可使用解密功能。\n\n\
                         （演示环境：本程序不具备任何解密或加密能力，文件并未被真正加密。）"
                        .into(),
                });
            }
        });

        if let Some(d) = clicked {
            self.dialog = Some(d);
        }
    }

    // ---------------------------------------------------------------- 提示框

    fn dialog_window(&mut self, ctx: &egui::Context) {
        let Some(d) = self.dialog.clone() else {
            return;
        };
        let mut close = false;
        egui::Window::new(&d.title)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_max_width(400.0);
                ui.label(RichText::new(&d.text).size(12.5));
                ui.add_space(12.0);
                ui.vertical_centered(|ui| {
                    if ui.button("确定").clicked() {
                        close = true;
                    }
                });
            });
        if close {
            self.dialog = None;
        }
    }
}

impl eframe::App for PopupApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        theme::OUTER_BORDER.to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // 倒计时需要持续刷新
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        // 收紧全局间距，让窗口看起来更像原版那个紧凑的老式界面
        ctx.all_styles_mut(|style| {
            style.spacing.item_spacing = Vec2::new(4.0, 4.0);
            style.spacing.button_padding = Vec2::new(6.0, 2.0);
        });

        // 外框：粉紫色粗边
        let outer = egui::Frame::NONE
            .fill(theme::OUTER_BORDER)
            .inner_margin(egui::Margin::same(6));

        egui::CentralPanel::no_frame().frame(outer).show(ui, |ui| {
            egui::Panel::top("titlebar")
                .exact_size(theme::TITLEBAR_H)
                .frame(egui::Frame::NONE.fill(theme::TITLEBAR_TOP))
                .show(ui, |ui| self.title_bar(ui, &ctx));

            egui::Panel::bottom("buttonbar")
                .exact_size(theme::BUTTONBAR_H)
                .frame(
                    egui::Frame::NONE
                        .fill(theme::WINDOW_BG)
                        .inner_margin(egui::Margin::symmetric(10, 11)),
                )
                .show(ui, |ui| self.button_bar(ui));

            egui::Panel::bottom("paybar")
                .exact_size(theme::PAYBAR_H)
                .frame(
                    egui::Frame::NONE
                        .fill(theme::PAYBAR_BG)
                        .inner_margin(egui::Margin::symmetric(10, 6)),
                )
                .show(ui, |ui| self.pay_bar(ui));

            egui::Panel::left("leftpanel")
                .exact_size(theme::LEFT_W)
                .resizable(false)
                .frame(
                    egui::Frame::NONE
                        .fill(theme::LEFT_PANEL_TOP)
                        .inner_margin(egui::Margin::same(8)),
                )
                .show(ui, |ui| self.left_panel(ui));

            egui::CentralPanel::no_frame()
                .frame(
                    egui::Frame::NONE
                        .fill(theme::WINDOW_BG)
                        .inner_margin(egui::Margin::same(9)),
                )
                .show(ui, |ui| self.body_panel(ui));
        });

        self.dialog_window(&ctx);
        self.handle_screenshot(&ctx);
    }
}

/// 把 egui 的截图保存成 PNG。
fn save_color_image(image: &egui::ColorImage, path: &Path) -> Result<()> {
    let [w, h] = image.size;
    let mut buf = image::RgbaImage::new(w as u32, h as u32);
    for (i, px) in image.pixels.iter().enumerate() {
        let x = (i % w) as u32;
        let y = (i / w) as u32;
        buf.put_pixel(x, y, image::Rgba([px.r(), px.g(), px.b(), px.a()]));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建目录失败：{}", parent.display()))?;
    }
    buf.save(path)
        .with_context(|| format!("保存 PNG 失败：{}", path.display()))?;
    Ok(())
}

/// 把剩余时间格式化成参考图里的 `DD:HH:MM:SS`。
fn fmt_time_left(deadline: DateTime<Local>) -> String {
    let delta = deadline - Local::now();
    let secs = delta.num_seconds();
    if secs <= 0 {
        return "00:00:00:00".to_string();
    }
    format!(
        "{:02}:{:02}:{:02}:{:02}",
        delta.num_days(),
        delta.num_hours() % 24,
        delta.num_minutes() % 60,
        secs % 60
    )
}

/// 垂直渐变填充。
fn paint_v_gradient(painter: &egui::Painter, rect: egui::Rect, top: Color32, bottom: Color32) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(2, 1, 3);
    painter.add(egui::Shape::mesh(mesh));
}
