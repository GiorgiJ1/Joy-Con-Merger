// joycon-merger/src/ui.rs
// egui desktop UI — industrial control panel aesthetic.

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::collections::VecDeque;
use crossbeam_channel::Receiver;
use eframe::{egui, App, CreationContext, Frame, NativeOptions};
use egui::{
    vec2, pos2, Color32, FontId, FontFamily, Rect, Rounding, Stroke,
    Align2, Shape, Pos2,
};
use anyhow::Result;

use crate::merger::StatusEvent;

// ── Palette ──────────────────────────────────────────────────────────────────
const BG:           Color32 = Color32::from_rgb(10,  11,  14);
const PANEL:        Color32 = Color32::from_rgb(16,  18,  22);
const PANEL2:       Color32 = Color32::from_rgb(20,  23,  28);
const BORDER:       Color32 = Color32::from_rgb(38,  42,  52);
const ACCENT:       Color32 = Color32::from_rgb(0,   210, 140); // Nintendo teal
const ACCENT_DIM:   Color32 = Color32::from_rgb(0,   90,  60);
const RED:          Color32 = Color32::from_rgb(255, 70,  70);
const RED_DIM:      Color32 = Color32::from_rgb(80,  20,  20);
const YELLOW:       Color32 = Color32::from_rgb(255, 200, 50);
const TEXT_HI:      Color32 = Color32::from_rgb(220, 225, 235);
const TEXT_MID:     Color32 = Color32::from_rgb(120, 130, 148);
const TEXT_LO:      Color32 = Color32::from_rgb(55,  62,  75);

// ── Connection state ─────────────────────────────────────────────────────────
#[derive(Clone, PartialEq)]
enum ConnState { Disconnected, Connected }

pub struct JoyConApp {
    running:   Arc<AtomicBool>,
    status_rx: Receiver<StatusEvent>,

    left_state:    ConnState,
    right_state:   ConnState,
    virtual_state: ConnState,

    log: VecDeque<String>,
    pulse: f32,   // 0..1 animation phase
    boot_t: f64,  // time at startup
}

impl JoyConApp {
    fn new(
        running: Arc<AtomicBool>,
        status_rx: Receiver<StatusEvent>,
        _cc: &CreationContext,
    ) -> Self {
        Self {
            running,
            status_rx,
            left_state:    ConnState::Disconnected,
            right_state:   ConnState::Disconnected,
            virtual_state: ConnState::Disconnected,
            log: VecDeque::with_capacity(64),
            pulse: 0.0,
            boot_t: 0.0,
        }
    }

    fn push_log(&mut self, msg: impl Into<String>) {
        let s = msg.into();
        if self.log.len() >= 60 { self.log.pop_front(); }
        self.log.push_back(s);
    }
}

impl App for JoyConApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // ── Drain status events ───────────────────────────────────────────
        // Collect first to release the immutable borrow on self.status_rx
        // before calling push_log (which needs &mut self).
        let events: Vec<StatusEvent> = self.status_rx.try_iter().collect();
        for ev in events {
            match ev {
                StatusEvent::LeftConnected     => { self.left_state    = ConnState::Connected;    self.push_log("▶ Left Joy-Con connected"); }
                StatusEvent::RightConnected    => { self.right_state   = ConnState::Connected;    self.push_log("▶ Right Joy-Con connected"); }
                StatusEvent::LeftDisconnected  => { self.left_state    = ConnState::Disconnected; self.push_log("◀ Left Joy-Con disconnected"); }
                StatusEvent::RightDisconnected => { self.right_state   = ConnState::Disconnected; self.push_log("◀ Right Joy-Con disconnected"); }
                StatusEvent::VirtualReady      => { self.virtual_state = ConnState::Connected;    self.push_log("● Virtual Xbox controller active"); }
                StatusEvent::VirtualDropped    => { self.virtual_state = ConnState::Disconnected; self.push_log("○ Virtual controller dropped"); }
                StatusEvent::InputError(e)     => { self.push_log(format!("! Error: {e}")); }
            }
        }

        // ── Animate ───────────────────────────────────────────────────────
        let t = ctx.input(|i| i.time);
        if self.boot_t == 0.0 { self.boot_t = t; }
        self.pulse = (t * 1.8).sin() as f32 * 0.5 + 0.5;
        ctx.request_repaint_after(std::time::Duration::from_millis(33));

        // ── Style setup ───────────────────────────────────────────────────
        let mut style = (*ctx.style()).clone();
        style.visuals.window_fill        = BG;
        style.visuals.panel_fill         = BG;
        style.visuals.override_text_color = Some(TEXT_HI);
        ctx.set_style(style);

        egui::CentralPanel::default()
            .frame(egui::containers::Frame::none().fill(BG))
            .show(ctx, |ui| {
                let full = ui.available_rect_before_wrap();
                let painter = ui.painter();

                // ── Background grid ───────────────────────────────────────
                draw_grid(painter, full);

                // ── Header bar ────────────────────────────────────────────
                let header_rect = Rect::from_min_size(full.min, vec2(full.width(), 56.0));
                painter.rect_filled(header_rect, Rounding::ZERO, PANEL);
                painter.rect_stroke(
                    Rect::from_min_max(
                        pos2(full.min.x, header_rect.max.y - 1.0),
                        pos2(full.max.x, header_rect.max.y),
                    ),
                    Rounding::ZERO,
                    Stroke::new(1.0, BORDER),
                );

                // Logo mark (geometric Joy-Con silhouette)
                draw_logo(painter, pos2(full.min.x + 28.0, header_rect.center().y));

                painter.text(
                    pos2(full.min.x + 56.0, header_rect.center().y - 8.0),
                    Align2::LEFT_CENTER,
                    "JOY-CON MERGER",
                    FontId::new(15.0, FontFamily::Monospace),
                    TEXT_HI,
                );
                painter.text(
                    pos2(full.min.x + 56.0, header_rect.center().y + 9.0),
                    Align2::LEFT_CENTER,
                    "VIRTUAL CONTROLLER BRIDGE  v0.1.0",
                    FontId::new(9.5, FontFamily::Monospace),
                    TEXT_MID,
                );

                // System status pill (top right)
                let all_ok = self.left_state == ConnState::Connected
                    && self.right_state == ConnState::Connected
                    && self.virtual_state == ConnState::Connected;
                let status_col = if all_ok { ACCENT } else { YELLOW };
                let status_txt = if all_ok { "ACTIVE" } else { "STANDBY" };
                let pill_x = full.max.x - 110.0;
                let pill_y = header_rect.center().y;
                let pill_r = Rect::from_center_size(pos2(pill_x, pill_y), vec2(90.0, 20.0));
                painter.rect_filled(pill_r, Rounding::same(10.0), Color32::from_rgba_premultiplied(
                    status_col.r(), status_col.g(), status_col.b(), 30));
                painter.rect_stroke(pill_r, Rounding::same(10.0), Stroke::new(1.0, status_col));
                // Blink dot
                let dot_alpha = if all_ok { (self.pulse * 255.0) as u8 } else { 180 };
                painter.circle_filled(
                    pos2(pill_x - 28.0, pill_y),
                    3.5,
                    Color32::from_rgba_premultiplied(status_col.r(), status_col.g(), status_col.b(), dot_alpha),
                );
                painter.text(pos2(pill_x - 18.0, pill_y), Align2::LEFT_CENTER,
                    status_txt, FontId::new(10.0, FontFamily::Monospace), status_col);

                // ── Main content area ─────────────────────────────────────
                let content_top = full.min.y + 56.0;
                let pad = 16.0;

                // Left column: device cards
                let col_w = (full.width() - pad * 3.0) * 0.55;
                let col_x = full.min.x + pad;

                // Joy-Con cards
                let card_h = 130.0;
                let gap = 12.0;

                let left_card = Rect::from_min_size(
                    pos2(col_x, content_top + pad),
                    vec2(col_w, card_h),
                );
                draw_device_card(
                    painter, left_card,
                    "LEFT JOY-CON", "HID  VID:057E  PID:2006",
                    &self.left_state, Side::Left, self.pulse,
                );

                let right_card = Rect::from_min_size(
                    pos2(col_x, content_top + pad + card_h + gap),
                    vec2(col_w, card_h),
                );
                draw_device_card(
                    painter, right_card,
                    "RIGHT JOY-CON", "HID  VID:057E  PID:2007",
                    &self.right_state, Side::Right, self.pulse,
                );

                // Virtual controller card
                let virt_card = Rect::from_min_size(
                    pos2(col_x, content_top + pad + (card_h + gap) * 2.0),
                    vec2(col_w, card_h),
                );
                draw_virtual_card(painter, virt_card, &self.virtual_state, self.pulse);

                // Right column: log panel
                let log_x = col_x + col_w + pad;
                let log_w = full.max.x - log_x - pad;
                let log_rect = Rect::from_min_size(
                    pos2(log_x, content_top + pad),
                    vec2(log_w, card_h * 3.0 + gap * 2.0),
                );
                draw_log_panel(painter, log_rect, &self.log);

                // ── Button mapping table ──────────────────────────────────
                let table_y = content_top + pad + (card_h + gap) * 3.0 + gap;
                let table_rect = Rect::from_min_size(
                    pos2(col_x, table_y),
                    vec2(full.width() - pad * 2.0, full.max.y - table_y - pad),
                );
                draw_mapping_table(painter, table_rect);

                // ── Footer ────────────────────────────────────────────────
                let footer_rect = Rect::from_min_size(
                    pos2(full.min.x, full.max.y - 28.0),
                    vec2(full.width(), 28.0),
                );
                painter.rect_filled(footer_rect, Rounding::ZERO, PANEL);
                painter.rect_stroke(
                    Rect::from_min_max(footer_rect.min, pos2(full.max.x, footer_rect.min.y + 1.0)),
                    Rounding::ZERO, Stroke::new(1.0, BORDER),
                );
                painter.text(
                    pos2(full.min.x + 14.0, footer_rect.center().y),
                    Align2::LEFT_CENTER,
                    "REQUIRES: VIGEMBUS DRIVER + BLUETOOTH",
                    FontId::new(9.0, FontFamily::Monospace),
                    TEXT_LO,
                );
                painter.text(
                    pos2(full.max.x - 14.0, footer_rect.center().y),
                    Align2::RIGHT_CENTER,
                    "PRESS  ENTER  TO  QUIT",
                    FontId::new(9.0, FontFamily::Monospace),
                    TEXT_LO,
                );

                // Quit on Enter
                if ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape)) {
                    self.running.store(false, Ordering::SeqCst);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Drawing helpers
// ─────────────────────────────────────────────────────────────────────────────

fn draw_grid(painter: &egui::Painter, rect: Rect) {
    let spacing = 32.0;
    let col = Color32::from_rgba_premultiplied(38, 42, 52, 18);
    let stroke = Stroke::new(0.5, col);
    let mut x = rect.min.x;
    while x <= rect.max.x {
        painter.line_segment([pos2(x, rect.min.y), pos2(x, rect.max.y)], stroke);
        x += spacing;
    }
    let mut y = rect.min.y;
    while y <= rect.max.y {
        painter.line_segment([pos2(rect.min.x, y), pos2(rect.max.x, y)], stroke);
        y += spacing;
    }
}

fn draw_logo(painter: &egui::Painter, center: Pos2) {
    // Left Joy-Con (L shape)
    let l_pts = vec![
        pos2(center.x - 14.0, center.y - 10.0),
        pos2(center.x - 8.0,  center.y - 10.0),
        pos2(center.x - 8.0,  center.y +  4.0),
        pos2(center.x - 2.0,  center.y +  4.0),
        pos2(center.x - 2.0,  center.y + 10.0),
        pos2(center.x - 14.0, center.y + 10.0),
    ];
    painter.add(Shape::convex_polygon(l_pts, ACCENT_DIM, Stroke::new(1.5, ACCENT)));

    // Right Joy-Con (mirrored)
    let r_pts = vec![
        pos2(center.x + 2.0,  center.y - 10.0),
        pos2(center.x + 14.0, center.y - 10.0),
        pos2(center.x + 14.0, center.y + 10.0),
        pos2(center.x + 2.0,  center.y + 10.0),
        pos2(center.x + 2.0,  center.y +  4.0),
        pos2(center.x + 8.0,  center.y +  4.0),
        pos2(center.x + 8.0,  center.y - 4.0),
        pos2(center.x + 2.0,  center.y - 4.0),
    ];
    painter.add(Shape::convex_polygon(r_pts, ACCENT_DIM, Stroke::new(1.5, ACCENT)));
}

#[derive(Clone, Copy)]
enum Side { Left, Right }

fn draw_device_card(
    painter: &egui::Painter,
    rect: Rect,
    title: &str,
    subtitle: &str,
    state: &ConnState,
    side: Side,
    pulse: f32,
) {
    let connected = *state == ConnState::Connected;
    let border_col = if connected { ACCENT } else { BORDER };
    let fill = if connected {
        Color32::from_rgba_premultiplied(0, 210, 140, 8)
    } else { PANEL };

    // Card background
    painter.rect_filled(rect, Rounding::same(4.0), fill);
    painter.rect_stroke(rect, Rounding::same(4.0), Stroke::new(1.0, border_col));

    // Corner accent (top-left)
    let ca = 10.0;
    painter.line_segment([rect.min, pos2(rect.min.x + ca, rect.min.y)], Stroke::new(2.0, border_col));
    painter.line_segment([rect.min, pos2(rect.min.x, rect.min.y + ca)], Stroke::new(2.0, border_col));

    // Joy-Con silhouette (geometric)
    let ico_center = pos2(rect.min.x + 48.0, rect.center().y);
    draw_joycon_shape(painter, ico_center, side, connected, pulse);

    // Text block
    let tx = rect.min.x + 96.0;
    let ty = rect.min.y + 22.0;

    painter.text(pos2(tx, ty), Align2::LEFT_TOP,
        title, FontId::new(13.0, FontFamily::Monospace), TEXT_HI);
    painter.text(pos2(tx, ty + 18.0), Align2::LEFT_TOP,
        subtitle, FontId::new(9.0, FontFamily::Monospace), TEXT_MID);

    // Status badge
    let badge_col   = if connected { ACCENT } else { RED };
    let badge_fill  = if connected { ACCENT_DIM } else { RED_DIM };
    let badge_label = if connected { "CONNECTED" } else { "WAITING" };
    let badge_rect  = Rect::from_min_size(pos2(tx, ty + 42.0), vec2(88.0, 18.0));
    painter.rect_filled(badge_rect, Rounding::same(2.0), badge_fill);
    painter.rect_stroke(badge_rect, Rounding::same(2.0), Stroke::new(1.0, badge_col));
    painter.text(badge_rect.center(), Align2::CENTER_CENTER,
        badge_label, FontId::new(9.5, FontFamily::Monospace), badge_col);

    // Pulse dot when connected
    if connected {
        let dot_alpha = (pulse * 200.0 + 55.0) as u8;
        painter.circle_filled(
            pos2(tx - 8.0, ty + 51.0),
            3.0,
            Color32::from_rgba_premultiplied(0, 210, 140, dot_alpha),
        );
    }

    // Data flow indicators (small channel lines)
    if connected {
        let line_y = rect.max.y - 22.0;
        let line_x0 = tx;
        let line_x1 = rect.max.x - 16.0;
        let seg_w = (line_x1 - line_x0) / 8.0;
        for i in 0..8 {
            let lx = line_x0 + i as f32 * seg_w;
            let alpha = if (i + (pulse * 8.0) as i32).rem_euclid(8) < 5 { 180u8 } else { 40u8 };
            painter.line_segment(
                [pos2(lx + 1.0, line_y), pos2(lx + seg_w - 2.0, line_y)],
                Stroke::new(1.5, Color32::from_rgba_premultiplied(0, 210, 140, alpha)),
            );
        }
        painter.text(pos2(line_x0, line_y + 8.0), Align2::LEFT_TOP,
            "INPUT STREAM", FontId::new(8.0, FontFamily::Monospace), TEXT_LO);
    }
}

fn draw_joycon_shape(painter: &egui::Painter, center: Pos2, side: Side, active: bool, pulse: f32) {
    let col = if active { ACCENT } else { TEXT_LO };
    let fill = if active {
        Color32::from_rgba_premultiplied(0, 210, 140, 20)
    } else {
        Color32::from_rgba_premultiplied(55, 62, 75, 30)
    };

    // Body rectangle
    let w = 22.0;
    let h = 56.0;
    let body = Rect::from_center_size(center, vec2(w, h));
    painter.rect_filled(body, Rounding::same(6.0), fill);
    painter.rect_stroke(body, Rounding::same(6.0), Stroke::new(1.5, col));

    // Analog stick circle
    let stick_y = match side { Side::Left => center.y - 10.0, Side::Right => center.y - 10.0 };
    let stick_x = match side { Side::Left => center.x - 3.0,  Side::Right => center.x + 3.0 };
    painter.circle_stroke(pos2(stick_x, stick_y), 7.0, Stroke::new(1.5, col));
    painter.circle_filled(pos2(stick_x, stick_y), 3.0, col);

    // Buttons (ABXY or dpad)
    let btn_cx = match side { Side::Left => center.x + 4.0, Side::Right => center.x - 4.0 };
    let btn_cy = center.y + 8.0;
    painter.circle_stroke(pos2(btn_cx,       btn_cy - 5.0), 2.5, Stroke::new(1.0, col));
    painter.circle_stroke(pos2(btn_cx - 5.0, btn_cy),       2.5, Stroke::new(1.0, col));
    painter.circle_stroke(pos2(btn_cx + 5.0, btn_cy),       2.5, Stroke::new(1.0, col));
    painter.circle_stroke(pos2(btn_cx,       btn_cy + 5.0), 2.5, Stroke::new(1.0, col));

    // Shoulder button
    let sh_rect = match side {
        Side::Left  => Rect::from_min_size(pos2(body.min.x, body.min.y - 7.0), vec2(w * 0.6, 6.0)),
        Side::Right => Rect::from_min_size(pos2(body.max.x - w * 0.6, body.min.y - 7.0), vec2(w * 0.6, 6.0)),
    };
    painter.rect_filled(sh_rect, Rounding::same(2.0), fill);
    painter.rect_stroke(sh_rect, Rounding::same(2.0), Stroke::new(1.0, col));

    // Pulse glow when active
    if active {
        let glow_alpha = (pulse * 25.0) as u8;
        painter.circle_filled(center, 36.0,
            Color32::from_rgba_premultiplied(0, 210, 140, glow_alpha));
    }
}

fn draw_virtual_card(
    painter: &egui::Painter,
    rect: Rect,
    state: &ConnState,
    pulse: f32,
) {
    let connected = *state == ConnState::Connected;
    let border_col = if connected { ACCENT } else { BORDER };
    let fill = if connected {
        Color32::from_rgba_premultiplied(0, 210, 140, 8)
    } else { PANEL };

    painter.rect_filled(rect, Rounding::same(4.0), fill);
    painter.rect_stroke(rect, Rounding::same(4.0), Stroke::new(1.0, border_col));

    let ca = 10.0;
    painter.line_segment([pos2(rect.max.x - ca, rect.min.y), rect.max_pos()], Stroke::new(2.0, border_col));
    painter.line_segment([pos2(rect.max.x, rect.min.y), pos2(rect.max.x, rect.min.y + ca)], Stroke::new(2.0, border_col));

    // Xbox controller icon
    let ico = pos2(rect.min.x + 48.0, rect.center().y);
    draw_xbox_shape(painter, ico, connected, pulse);

    let tx = rect.min.x + 96.0;
    let ty = rect.min.y + 22.0;

    painter.text(pos2(tx, ty), Align2::LEFT_TOP,
        "VIRTUAL XBOX 360", FontId::new(13.0, FontFamily::Monospace), TEXT_HI);
    painter.text(pos2(tx, ty + 18.0), Align2::LEFT_TOP,
        "VIGEMBUS  XINPUT  WIRED", FontId::new(9.0, FontFamily::Monospace), TEXT_MID);

    let badge_col   = if connected { ACCENT } else { YELLOW };
    let badge_fill  = if connected { ACCENT_DIM } else { Color32::from_rgb(60, 46, 10) };
    let badge_label = if connected { "ACTIVE" } else { "AWAITING INPUT" };
    let badge_w     = if connected { 64.0 } else { 110.0 };
    let badge_rect  = Rect::from_min_size(pos2(tx, ty + 42.0), vec2(badge_w, 18.0));
    painter.rect_filled(badge_rect, Rounding::same(2.0), badge_fill);
    painter.rect_stroke(badge_rect, Rounding::same(2.0), Stroke::new(1.0, badge_col));
    painter.text(badge_rect.center(), Align2::CENTER_CENTER,
        badge_label, FontId::new(9.5, FontFamily::Monospace), badge_col);

    if connected {
        let dot_alpha = (pulse * 200.0 + 55.0) as u8;
        painter.circle_filled(
            pos2(tx - 8.0, ty + 51.0), 3.0,
            Color32::from_rgba_premultiplied(0, 210, 140, dot_alpha),
        );
    }
}

fn draw_xbox_shape(painter: &egui::Painter, center: Pos2, active: bool, pulse: f32) {
    let col = if active { ACCENT } else { TEXT_LO };
    let fill = if active {
        Color32::from_rgba_premultiplied(0, 210, 140, 20)
    } else {
        Color32::from_rgba_premultiplied(55, 62, 75, 30)
    };

    // Controller body (oval-ish via rect + circles)
    let body = Rect::from_center_size(center, vec2(46.0, 32.0));
    painter.rect_filled(body, Rounding::same(14.0), fill);
    painter.rect_stroke(body, Rounding::same(14.0), Stroke::new(1.5, col));

    // Left grip
    painter.rect_filled(
        Rect::from_center_size(pos2(center.x - 14.0, center.y + 14.0), vec2(12.0, 16.0)),
        Rounding::same(5.0), fill,
    );
    painter.rect_stroke(
        Rect::from_center_size(pos2(center.x - 14.0, center.y + 14.0), vec2(12.0, 16.0)),
        Rounding::same(5.0), Stroke::new(1.5, col),
    );
    // Right grip
    painter.rect_filled(
        Rect::from_center_size(pos2(center.x + 14.0, center.y + 14.0), vec2(12.0, 16.0)),
        Rounding::same(5.0), fill,
    );
    painter.rect_stroke(
        Rect::from_center_size(pos2(center.x + 14.0, center.y + 14.0), vec2(12.0, 16.0)),
        Rounding::same(5.0), Stroke::new(1.5, col),
    );

    // Guide button (center circle)
    painter.circle_stroke(center, 5.0, Stroke::new(1.5, col));

    // Left stick
    painter.circle_stroke(pos2(center.x - 14.0, center.y - 2.0), 5.0, Stroke::new(1.0, col));
    // Right stick
    painter.circle_stroke(pos2(center.x + 8.0, center.y + 4.0), 5.0, Stroke::new(1.0, col));

    // ABXY dots
    painter.circle_filled(pos2(center.x + 18.0, center.y - 5.0), 2.0, col);
    painter.circle_filled(pos2(center.x + 23.0, center.y),        2.0, col);
    painter.circle_filled(pos2(center.x + 18.0, center.y + 5.0),  2.0, col);
    painter.circle_filled(pos2(center.x + 13.0, center.y),        2.0, col);

    if active {
        let glow_alpha = (pulse * 30.0) as u8;
        painter.circle_filled(center, 38.0,
            Color32::from_rgba_premultiplied(0, 210, 140, glow_alpha));
    }
}

fn draw_log_panel(painter: &egui::Painter, rect: Rect, log: &VecDeque<String>) {
    painter.rect_filled(rect, Rounding::same(4.0), PANEL2);
    painter.rect_stroke(rect, Rounding::same(4.0), Stroke::new(1.0, BORDER));

    // Header
    let hdr = Rect::from_min_size(rect.min, vec2(rect.width(), 24.0));
    painter.rect_filled(hdr, Rounding { nw: 4.0, ne: 4.0, sw: 0.0, se: 0.0 }, PANEL);
    painter.rect_stroke(
        Rect::from_min_max(pos2(rect.min.x, hdr.max.y - 1.0), pos2(rect.max.x, hdr.max.y)),
        Rounding::ZERO, Stroke::new(1.0, BORDER),
    );
    painter.text(pos2(rect.min.x + 10.0, hdr.center().y), Align2::LEFT_CENTER,
        "EVENT LOG", FontId::new(9.5, FontFamily::Monospace), TEXT_MID);

    // Scan line decoration
    painter.circle_filled(pos2(rect.max.x - 12.0, hdr.center().y), 3.0, ACCENT_DIM);
    painter.circle_stroke(pos2(rect.max.x - 12.0, hdr.center().y), 3.0, Stroke::new(1.0, ACCENT));

    // Log lines (most recent at bottom)
    let line_h = 14.0;
    let max_lines = ((rect.height() - 30.0) / line_h) as usize;
    let start = if log.len() > max_lines { log.len() - max_lines } else { 0 };

    for (i, line) in log.iter().skip(start).enumerate() {
        let y = rect.min.y + 30.0 + i as f32 * line_h;

        // Color code by prefix
        let col = if line.starts_with('▶') { ACCENT }
            else if line.starts_with('●') { ACCENT }
            else if line.starts_with('◀') || line.starts_with('○') { YELLOW }
            else if line.starts_with('!') { RED }
            else { TEXT_MID };

        // Line number
        painter.text(
            pos2(rect.min.x + 8.0, y),
            Align2::LEFT_TOP,
            &format!("{:02}", start + i + 1),
            FontId::new(8.5, FontFamily::Monospace),
            TEXT_LO,
        );

        painter.text(
            pos2(rect.min.x + 28.0, y),
            Align2::LEFT_TOP,
            line,
            FontId::new(9.5, FontFamily::Monospace),
            col,
        );
    }

    if log.is_empty() {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "— no events yet —",
            FontId::new(9.5, FontFamily::Monospace),
            TEXT_LO,
        );
    }
}

fn draw_mapping_table(painter: &egui::Painter, rect: Rect) {
    if rect.height() < 40.0 { return; }

    painter.rect_filled(rect, Rounding::same(4.0), PANEL2);
    painter.rect_stroke(rect, Rounding::same(4.0), Stroke::new(1.0, BORDER));

    let hdr = Rect::from_min_size(rect.min, vec2(rect.width(), 24.0));
    painter.rect_filled(hdr, Rounding { nw: 4.0, ne: 4.0, sw: 0.0, se: 0.0 }, PANEL);
    painter.rect_stroke(
        Rect::from_min_max(pos2(rect.min.x, hdr.max.y - 1.0), pos2(rect.max.x, hdr.max.y)),
        Rounding::ZERO, Stroke::new(1.0, BORDER),
    );
    painter.text(pos2(rect.min.x + 10.0, hdr.center().y), Align2::LEFT_CENTER,
        "BUTTON MAP", FontId::new(9.5, FontFamily::Monospace), TEXT_MID);

    let mappings: &[(&str, &str, &str)] = &[
        ("LEFT",  "D-PAD  ↑↓←→",    "DPAD"),
        ("LEFT",  "−  BUTTON",       "BACK / SELECT"),
        ("LEFT",  "L  SHOULDER",     "LB"),
        ("LEFT",  "ZL  TRIGGER",     "LT  (ANALOG)"),
        ("LEFT",  "LEFT  STICK",     "LS  +  L3"),
        ("RIGHT", "A  B  X  Y",      "A  B  X  Y"),
        ("RIGHT", "+  BUTTON",       "START"),
        ("RIGHT", "R  SHOULDER",     "RB"),
        ("RIGHT", "ZR  TRIGGER",     "RT  (ANALOG)"),
        ("RIGHT", "RIGHT  STICK",    "RS  +  R3"),
        ("RIGHT", "HOME  BUTTON",    "GUIDE"),
    ];

    let cols = 3usize;  // show 3 mappings per row
    let row_h = 16.0;
    let col_w = rect.width() / cols as f32;

    for (i, (side, from, to)) in mappings.iter().enumerate() {
        let col_i = i % cols;
        let row_i = i / cols;

        let x = rect.min.x + col_i as f32 * col_w + 10.0;
        let y = rect.min.y + 28.0 + row_i as f32 * row_h;

        let side_col = if *side == "LEFT" {
            Color32::from_rgb(90, 180, 255)
        } else {
            Color32::from_rgb(255, 120, 80)
        };

        painter.text(pos2(x, y), Align2::LEFT_TOP,
            side, FontId::new(7.5, FontFamily::Monospace), side_col);
        painter.text(pos2(x + 36.0, y), Align2::LEFT_TOP,
            from, FontId::new(8.5, FontFamily::Monospace), TEXT_HI);
        painter.text(pos2(x + 36.0 + col_w * 0.38, y), Align2::LEFT_TOP,
            "→", FontId::new(8.5, FontFamily::Monospace), TEXT_LO);
        painter.text(pos2(x + 36.0 + col_w * 0.42, y), Align2::LEFT_TOP,
            to, FontId::new(8.5, FontFamily::Monospace), ACCENT);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public entry point
// ─────────────────────────────────────────────────────────────────────────────

pub fn run(running: Arc<AtomicBool>, status_rx: Receiver<StatusEvent>) -> Result<()> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Joy-Con Merger")
            .with_inner_size([860.0, 620.0])
            .with_min_inner_size([700.0, 500.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "Joy-Con Merger",
        options,
        Box::new(move |cc| -> Box<dyn eframe::App> {
            Box::new(JoyConApp::new(running, status_rx, cc))
        }),
    ).map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    Ok(())
}

// Helper trait for Rect corners
trait RectExt { fn max_pos(&self) -> Pos2; }
impl RectExt for Rect {
    fn max_pos(&self) -> Pos2 { pos2(self.max.x - 10.0, self.min.y) }
}