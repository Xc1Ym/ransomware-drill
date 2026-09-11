//! 弹窗配色。
//!
//! 取自参考图（Wana Decrypt0r 2.0 窗口）的主色调：粉紫外框 + 紫色标题栏 +
//! 深红左栏 + 白色正文区 + 橙色比特币标识。

use egui::Color32;

/// 窗口外框：浅粉紫。
pub const OUTER_BORDER: Color32 = Color32::from_rgb(0xE8, 0xB4, 0xD8);

/// 标题栏：紫色渐变的上端与下端。
pub const TITLEBAR_TOP: Color32 = Color32::from_rgb(0xC8, 0x6A, 0xA8);
pub const TITLEBAR_BOTTOM: Color32 = Color32::from_rgb(0x9C, 0x3C, 0x80);

/// 窗口主体底色。
pub const WINDOW_BG: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);

/// 左栏：深红渐变。
pub const LEFT_PANEL_TOP: Color32 = Color32::from_rgb(0xA8, 0x00, 0x00);
pub const LEFT_PANEL_BOTTOM: Color32 = Color32::from_rgb(0x70, 0x00, 0x00);

/// 正文区顶部的红色标题条。
pub const HEADER_RED: Color32 = Color32::from_rgb(0xC0, 0x00, 0x00);

/// 锁图标：浅粉底 + 深红图案。
pub const LOCK_BG: Color32 = Color32::from_rgb(0xF5, 0xD8, 0xD8);
pub const LOCK_FG: Color32 = Color32::from_rgb(0xC0, 0x00, 0x00);

/// 倒计时框：白底红字。
pub const COUNTDOWN_BG: Color32 = Color32::WHITE;
pub const TIMER_FG: Color32 = Color32::from_rgb(0xC0, 0x00, 0x00);
pub const COUNTDOWN_BORDER: Color32 = Color32::from_rgb(0x88, 0x00, 0x00);

/// 正文区顶部的红色标题条。
pub const BODY_BORDER: Color32 = Color32::from_rgb(0x40, 0x40, 0x40);
pub const BODY_BG: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
pub const TEXT_DARK: Color32 = Color32::from_rgb(0x1A, 0x1A, 0x1A);

/// 底部的付款信息条。
pub const PAYBAR_BG: Color32 = Color32::from_rgb(0x8B, 0x1A, 0x1A);
pub const BITCOIN_ORANGE: Color32 = Color32::from_rgb(0xF7, 0x93, 0x1A);

/// 链接与按钮。
pub const LINK_BLUE: Color32 = Color32::from_rgb(0x10, 0x30, 0xC0);
pub const BUTTON_BG: Color32 = Color32::from_rgb(0xF0, 0xF0, 0xF0);
pub const BUTTON_FG: Color32 = Color32::from_rgb(0x10, 0x10, 0x10);

/// 顶部标题栏高度。
pub const TITLEBAR_H: f32 = 28.0;
/// 左栏宽度。
pub const LEFT_W: f32 = 216.0;
/// 底部按钮条高度。
pub const BUTTONBAR_H: f32 = 54.0;
/// 付款信息条高度。
pub const PAYBAR_H: f32 = 62.0;
/// 窗口默认尺寸。
pub const WINDOW_W: f32 = 880.0;
pub const WINDOW_H: f32 = 600.0;
