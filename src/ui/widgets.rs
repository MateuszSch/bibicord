//! Reusable components built on the tokens in `theme`.

use super::theme::{self, W, c, f, mix, r, sp, ty};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, FontId, Pos2, Rect, Response, RichText,
    Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2, text::LayoutJob,
};
pub use egui_phosphor::regular as ic;

/// Animated 0..1 hover factor for a response.
pub fn hover_t(ui: &Ui, resp: &Response) -> f32 {
    ui.ctx().animate_bool_with_time(resp.id.with("hover"), resp.hovered(), theme::ANIM)
}

pub fn text(s: impl Into<String>, (size, w): (f32, W), color: Color32) -> RichText {
    RichText::new(s).font(theme::font(size, w)).color(color)
}

pub fn icon(code: &str, size: f32, color: Color32) -> RichText {
    RichText::new(code).font(theme::icon_font(size)).color(color)
}

/// Paints a glyph centred at `pos`.
pub fn paint_icon(ui: &Ui, pos: Pos2, code: &str, size: f32, color: Color32, fill: bool) {
    let font = if fill { theme::icon_fill_font(size) } else { theme::icon_font(size) };
    ui.painter().text(pos, Align2::CENTER_CENTER, code, font, color);
}

/// Category header in the sidebar: small, uppercase, tracked.
pub fn section_label(ui: &mut Ui, label: &str) {
    let mut job = LayoutJob::default();
    job.append(
        &label.to_uppercase(),
        0.0,
        egui::TextFormat {
            font_id: f(ty::SECTION),
            color: c::TEXT_MUTED,
            extra_letter_spacing: 0.6,
            ..Default::default()
        },
    );
    ui.add_space(sp::L);
    ui.horizontal(|ui| {
        ui.add_space(sp::S);
        ui.label(job);
    });
    ui.add_space(sp::XXS);
}

// ------------------------------------------------------------------ avatar

pub struct Avatar<'a> {
    pub name: &'a str,
    pub color: Color32,
    /// Profile picture; the coloured initial is drawn when there is none.
    pub image: Option<&'a egui::TextureHandle>,
    pub radius: f32,
    /// 0..1, animated speaking ring.
    pub speaking: f32,
    /// Status dot colour and the surface it sits on (for the cut-out).
    pub status: Option<(Color32, Color32)>,
}

impl Avatar<'_> {
    pub fn paint(&self, ui: &Ui, center: Pos2) {
        let p = ui.painter();
        if let Some(tex) = self.image {
            let rect = Rect::from_center_size(center, Vec2::splat(self.radius * 2.0));
            egui::Image::from_texture((tex.id(), rect.size()))
                .corner_radius(self.radius.min(255.0) as u8)
                .paint_at(ui, rect);
        } else {
            p.circle_filled(center, self.radius, self.color);
            let initial: String = self
                .name
                .trim()
                .chars()
                .next()
                .map(|ch| ch.to_uppercase().collect())
                .unwrap_or_else(|| "?".into());
            p.text(
                center + Vec2::new(0.0, 0.5),
                Align2::CENTER_CENTER,
                initial,
                theme::font((self.radius * 0.95).max(10.0), W::SemiBold),
                c::WHITE,
            );
        }
        if self.speaking > 0.0 {
            p.circle_stroke(
                center,
                self.radius + 2.5,
                Stroke::new(2.0, theme::alpha(c::SPEAKING, self.speaking)),
            );
        }
        if let Some((dot, surface)) = self.status {
            let d = (self.radius * 0.36).max(4.0);
            let at = center + Vec2::splat(self.radius * 0.72);
            p.circle_filled(at, d + 2.5, surface);
            p.circle_filled(at, d, dot);
        }
    }

    /// Allocates space and paints. Returns the response for tooltips.
    pub fn show(&self, ui: &mut Ui) -> Response {
        let size = Vec2::splat(self.radius * 2.0 + 6.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        self.paint(ui, rect.center());
        resp
    }
}

// ----------------------------------------------------------------- badges

/// Red "NA ŻYWO" pill. Returns its rect.
pub fn live_badge(ui: &Ui, anchor: Pos2, align: Align2, small: bool) -> Rect {
    let font = theme::font(if small { 10.0 } else { 11.0 }, W::Bold);
    let galley = ui.painter().layout_no_wrap("NA ŻYWO".into(), font, c::WHITE);
    let pad = if small { Vec2::new(5.0, 2.0) } else { Vec2::new(6.0, 3.0) };
    let size = galley.size() + pad * 2.0;
    let rect = align.anchor_size(anchor, size);
    ui.painter().rect_filled(rect, r::BADGE, c::DANGER);
    ui.painter().galley(rect.min + pad, galley, c::WHITE);
    rect
}

/// Allocating version of the live badge for horizontal layouts.
pub fn live_badge_inline(ui: &mut Ui) -> Response {
    let font = theme::font(10.0, W::Bold);
    let galley = ui.painter().layout_no_wrap("NA ŻYWO".into(), font, c::WHITE);
    let size = galley.size() + Vec2::new(10.0, 4.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    live_badge(ui, rect.center(), Align2::CENTER_CENTER, true);
    resp
}

pub fn count_badge(ui: &mut Ui, n: usize) {
    let label = if n > 99 { "99+".to_string() } else { n.to_string() };
    let galley = ui.painter().layout_no_wrap(label, theme::font(11.0, W::Bold), c::WHITE);
    let size = Vec2::new((galley.size().x + 10.0).max(18.0), 18.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(9), c::DANGER);
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, c::WHITE);
}

// ---------------------------------------------------------------- buttons

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Primary,
    Secondary,
    Danger,
    Ghost,
    Success,
}

impl Kind {
    fn colors(self) -> (Color32, Color32, Color32) {
        // (fill, hover fill, text)
        match self {
            Kind::Primary => (c::BRAND, c::BRAND_HOVER, c::ON_BRAND),
            Kind::Secondary => (c::BG_SELECTED, c::BG_PRESSED, c::TEXT),
            Kind::Danger => (c::DANGER, c::DANGER_HOVER, c::WHITE),
            Kind::Ghost => (Color32::TRANSPARENT, c::BG_SELECTED, c::TEXT_SECONDARY),
            Kind::Success => (theme::alpha(c::SUCCESS, 0.16), theme::alpha(c::SUCCESS, 0.24), c::SUCCESS),
        }
    }
}

pub struct Button<'a> {
    pub label: &'a str,
    pub icon: Option<&'a str>,
    pub kind: Kind,
    pub width: Option<f32>,
    pub height: f32,
    pub enabled: bool,
}

impl<'a> Button<'a> {
    pub fn new(label: &'a str, kind: Kind) -> Self {
        Button { label, icon: None, kind, width: None, height: 38.0, enabled: true }
    }
    pub fn icon(mut self, icon: &'a str) -> Self {
        self.icon = Some(icon);
        self
    }
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    pub fn enabled(mut self, e: bool) -> Self {
        self.enabled = e;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let font = f(ty::LABEL);
        let galley = ui.painter().layout_no_wrap(self.label.into(), font, c::TEXT);
        let icon_w = if self.icon.is_some() { 18.0 + 8.0 } else { 0.0 };
        let content_w = galley.size().x + icon_w;
        let w = self.width.unwrap_or(content_w + 32.0).max(content_w + 24.0);
        let sense = if self.enabled { Sense::click() } else { Sense::hover() };
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, self.height), sense);
        let t = hover_t(ui, &resp);
        let (fill, hover, mut fg) = self.kind.colors();
        let mut bg = mix(fill, hover, t);
        if resp.is_pointer_button_down_on() && self.enabled {
            bg = mix(bg, Color32::BLACK, 0.12);
        }
        if !self.enabled {
            bg = theme::alpha(bg, 0.45);
            fg = theme::alpha(fg, 0.45);
        }
        if self.kind == Kind::Ghost && !matches!(self.icon, Some(_)) && t < 0.01 {
            ui.painter()
                .rect_stroke(rect, r::CONTROL, Stroke::new(1.0, c::BORDER_STRONG), StrokeKind::Inside);
        }
        ui.painter().rect_filled(rect, r::CONTROL, bg);
        let mut x = rect.center().x - content_w / 2.0;
        if let Some(i) = self.icon {
            paint_icon(ui, Pos2::new(x + 9.0, rect.center().y), i, 18.0, fg, false);
            x += icon_w;
        }
        let ky = rect.center().y - galley.size().y / 2.0;
        // The galley was laid out before the state colour was known.
        ui.painter().galley_with_override_text_color(Pos2::new(x, ky), galley, fg);
        if self.enabled {
            resp.on_hover_cursor(CursorIcon::PointingHand)
        } else {
            resp
        }
    }
}

/// Square icon-only button used in panels and headers.
pub struct IconButton<'a> {
    pub icon: &'a str,
    pub tooltip: &'a str,
    pub size: f32,
    pub color: Color32,
    pub active: bool,
}

impl<'a> IconButton<'a> {
    pub fn new(icon: &'a str, tooltip: &'a str) -> Self {
        IconButton { icon, tooltip, size: 32.0, color: c::TEXT_SECONDARY, active: false }
    }
    pub fn color(mut self, col: Color32) -> Self {
        self.color = col;
        self
    }
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    pub fn show(self, ui: &mut Ui) -> Response {
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(self.size), Sense::click());
        let t = hover_t(ui, &resp);
        let base = if self.active { c::BG_SELECTED } else { Color32::TRANSPARENT };
        ui.painter().rect_filled(rect, r::ROW, mix(base, c::BG_SELECTED, t));
        let fg = if self.color == c::TEXT_SECONDARY { mix(c::TEXT_SECONDARY, c::TEXT, t) } else { self.color };
        paint_icon(ui, rect.center(), self.icon, self.size * 0.6, fg, false);
        resp.on_hover_text(self.tooltip).on_hover_cursor(CursorIcon::PointingHand)
    }
}

/// Round control used on the voice stage.
pub fn round_button(ui: &mut Ui, icon_code: &str, tooltip: &str, kind: RoundKind) -> Response {
    let size = 48.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let t = hover_t(ui, &resp);
    let (bg, hover, fg) = match kind {
        RoundKind::Normal => (c::BG_SELECTED, c::BG_PRESSED, c::TEXT),
        RoundKind::Off => (mix(c::BG_SELECTED, c::DANGER, 0.22), mix(c::BG_SELECTED, c::DANGER, 0.34), mix(c::DANGER, c::WHITE, 0.2)),
        RoundKind::On => (c::BRAND, c::BRAND_HOVER, c::ON_BRAND),
        RoundKind::Hangup => (c::DANGER, c::DANGER_HOVER, c::WHITE),
    };
    ui.painter().circle_filled(rect.center(), size / 2.0, mix(bg, hover, t));
    paint_icon(ui, rect.center(), icon_code, 22.0, fg, matches!(kind, RoundKind::Hangup));
    resp.on_hover_text(tooltip).on_hover_cursor(CursorIcon::PointingHand)
}

#[derive(Clone, Copy)]
pub enum RoundKind {
    Normal,
    Off,
    On,
    Hangup,
}

/// Selectable chip for option groups in dialogs.
pub fn chip(ui: &mut Ui, label: &str, selected: bool) -> Response {
    let galley = ui.painter().layout_no_wrap(label.into(), f(ty::SMALL_MEDIUM), c::TEXT);
    let size = Vec2::new(galley.size().x + 24.0, 32.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let t = hover_t(ui, &resp);
    let (bg, fg) = if selected {
        (theme::alpha(c::BRAND, 0.18), c::TEXT)
    } else {
        (mix(c::BG_RAISED, c::BG_SELECTED, t), mix(c::TEXT_SECONDARY, c::TEXT, t))
    };
    ui.painter().rect_filled(rect, r::CONTROL, bg);
    if selected {
        ui.painter()
            .rect_stroke(rect, r::CONTROL, Stroke::new(1.0, c::BRAND), StrokeKind::Inside);
    }
    ui.painter().galley_with_override_text_color(rect.center() - galley.size() / 2.0, galley, fg);
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

// ------------------------------------------------------------------- rows

/// A full-width interactive list row. Returns (response, rect, hover t).
pub fn row(ui: &mut Ui, height: f32, selected: bool) -> (Response, Rect, f32) {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, height), Sense::click());
    let t = hover_t(ui, &resp);
    let bg = if selected {
        c::BG_SELECTED
    } else {
        theme::alpha(c::BG_RAISED, t)
    };
    ui.painter().rect_filled(rect, r::ROW, bg);
    (resp.on_hover_cursor(CursorIcon::PointingHand), rect, t)
}

pub fn paint_text(ui: &Ui, pos: Pos2, align: Align2, s: &str, font: FontId, color: Color32) -> Rect {
    ui.painter().text(pos, align, s, font, color)
}

/// Text clipped with an ellipsis to `max_w`.
pub fn paint_text_clipped(ui: &Ui, pos: Pos2, s: &str, font: FontId, color: Color32, max_w: f32) -> Rect {
    let mut job = LayoutJob::simple_singleline(s.to_string(), font, color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(max_w.max(10.0));
    let galley = ui.painter().layout_job(job);
    let rect = Rect::from_min_size(Pos2::new(pos.x, pos.y - galley.size().y / 2.0), galley.size());
    ui.painter().galley(rect.min, galley, color);
    rect
}

// ----------------------------------------------------------------- inputs

pub fn input_frame(focused: bool) -> egui::Frame {
    egui::Frame::new()
        .fill(c::BG_RAISED)
        .corner_radius(r::CONTROL)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .stroke(if focused { Stroke::new(1.0, c::BRAND) } else { Stroke::new(1.0, c::BG_RAISED) })
}

/// Single-line text field with the design system's frame.
pub fn text_field(ui: &mut Ui, value: &mut String, hint: &str, id: &str, width: f32) -> Response {
    let id = ui.make_persistent_id(id);
    let focused = ui.memory(|m| m.has_focus(id));
    TextEdit::singleline(value)
        .id(id)
        .hint_text(text(hint, ty::BODY, c::TEXT_MUTED))
        .font(f(ty::BODY))
        .text_color(c::TEXT)
        .frame(input_frame(focused))
        .margin(egui::Margin::symmetric(12, 10))
        .desired_width(width)
        .show(ui)
        .response
        .response
}

/// Label above a field.
pub fn field_label(ui: &mut Ui, label: &str) {
    let mut job = LayoutJob::default();
    job.append(
        &label.to_uppercase(),
        0.0,
        egui::TextFormat {
            font_id: f(ty::SECTION),
            color: c::TEXT_SECONDARY,
            extra_letter_spacing: 0.6,
            ..Default::default()
        },
    );
    ui.label(job);
    ui.add_space(sp::XS);
}

/// Field label with a value aligned to the right on the same line.
pub fn field_label_value(ui: &mut Ui, label: &str, value: &str) {
    let mut job = LayoutJob::default();
    job.append(
        &label.to_uppercase(),
        0.0,
        egui::TextFormat {
            font_id: f(ty::SECTION),
            color: c::TEXT_SECONDARY,
            extra_letter_spacing: 0.6,
            ..Default::default()
        },
    );
    let galley = ui.painter().layout_job(job);
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 18.0), Sense::hover());
    ui.painter()
        .galley(Pos2::new(rect.left(), rect.center().y - galley.size().y / 2.0), galley, c::TEXT_SECONDARY);
    paint_text(ui, rect.right_center(), Align2::RIGHT_CENTER, value, f(ty::SMALL_MEDIUM), c::TEXT);
    ui.add_space(sp::XS);
}

pub fn separator(ui: &mut Ui, color: Color32) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, color);
}

// ---------------------------------------------------------------- overlays

pub fn modal_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(c::BG_CONTENT)
        .corner_radius(r::CARD)
        .shadow(theme::shadow_overlay())
}

pub fn menu_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(c::BG_BASE)
        .corner_radius(r::CONTROL)
        .shadow(theme::shadow_popup())
        .inner_margin(egui::Margin::same(6))
}

/// Item inside a dropdown menu.
pub fn menu_item(ui: &mut Ui, label: &str, icon_code: &str, danger: bool) -> Response {
    let (resp, rect, t) = {
        let w = ui.available_width().max(200.0);
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 34.0), Sense::click());
        let t = hover_t(ui, &resp);
        (resp, rect, t)
    };
    let hover_bg = if danger { c::DANGER } else { c::BRAND };
    ui.painter().rect_filled(rect, r::ROW, theme::alpha(hover_bg, t));
    let fg = if danger { mix(c::DANGER, c::WHITE, t) } else { mix(c::TEXT_SECONDARY, c::ON_BRAND, t) };
    paint_text(ui, rect.left_center() + Vec2::new(10.0, 0.0), Align2::LEFT_CENTER, label, f(ty::SMALL_MEDIUM), fg);
    paint_icon(ui, rect.right_center() - Vec2::new(18.0, 0.0), icon_code, 17.0, fg, false);
    resp.on_hover_cursor(CursorIcon::PointingHand)
}
