//! Design tokens: colour, type, spacing, radii, elevation.
//! The rules behind these values live in DESIGN.md.

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Shadow, Stroke,
    Vec2,
};
use std::sync::Arc;

// ------------------------------------------------------------------ colour

pub mod c {
    use super::Color32;
    const fn hex(v: u32) -> Color32 {
        Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
    }
    // Surfaces, darkest to lightest.
    pub const BG_STAGE: Color32 = hex(0x0b0c0e);
    pub const BG_BASE: Color32 = hex(0x111214);
    pub const BG_SIDEBAR: Color32 = hex(0x16171a);
    pub const BG_CONTENT: Color32 = hex(0x1c1d21);
    pub const BG_HOVER_SOFT: Color32 = hex(0x202125);
    pub const BG_RAISED: Color32 = hex(0x232428);
    pub const BG_SELECTED: Color32 = hex(0x2b2d32);
    pub const BG_PRESSED: Color32 = hex(0x34363c);
    pub const BORDER: Color32 = hex(0x2b2d32);
    pub const BORDER_STRONG: Color32 = hex(0x3a3c42);
    // Text.
    pub const TEXT: Color32 = hex(0xf2f3f5);
    pub const TEXT_BODY: Color32 = hex(0xdbdee1);
    pub const TEXT_SECONDARY: Color32 = hex(0xb5bac1);
    pub const TEXT_MUTED: Color32 = hex(0x949ba4);
    // Intent.
    pub const BRAND: Color32 = hex(0x5b6cff);
    pub const BRAND_HOVER: Color32 = hex(0x4c5cf0);
    pub const SUCCESS: Color32 = hex(0x2dc770);
    pub const DANGER: Color32 = hex(0xed4b55);
    pub const DANGER_HOVER: Color32 = hex(0xd83c46);
    pub const WARNING: Color32 = hex(0xf0b232);
    pub const WHITE: Color32 = Color32::WHITE;

    /// Avatar hues, picked deterministically per user.
    pub const AVATARS: [Color32; 8] = [
        hex(0x5b6cff),
        hex(0xe0577a),
        hex(0x2fa37a),
        hex(0xe08a3c),
        hex(0x9a5be0),
        hex(0x3c9ee0),
        hex(0xc9a227),
        hex(0xd2554a),
    ];
}

/// Linear blend between two colours (t = 0 → a, t = 1 → b).
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

pub fn alpha(col: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), (a.clamp(0.0, 1.0) * 255.0) as u8)
}

pub fn avatar_color(seed: &[u8]) -> Color32 {
    // FNV-1a: stable across runs and peers.
    let mut h: u32 = 0x811c9dc5;
    for b in seed {
        h ^= *b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    c::AVATARS[(h % c::AVATARS.len() as u32) as usize]
}

// -------------------------------------------------------------- typography

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum W {
    Regular,
    Medium,
    SemiBold,
    Bold,
}

pub fn font(size: f32, w: W) -> FontId {
    let fam = match w {
        W::Regular => FontFamily::Proportional,
        W::Medium => FontFamily::Name("medium".into()),
        W::SemiBold => FontFamily::Name("semibold".into()),
        W::Bold => FontFamily::Name("bold".into()),
    };
    FontId::new(size, fam)
}

pub fn icon_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("icon".into()))
}

pub fn icon_fill_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("icon-fill".into()))
}

/// Type scale (size, weight).
pub mod ty {
    use super::W;
    pub const DISPLAY: (f32, W) = (26.0, W::Bold);
    pub const TITLE: (f32, W) = (18.0, W::SemiBold);
    pub const HEADING: (f32, W) = (15.0, W::SemiBold);
    pub const BODY: (f32, W) = (15.0, W::Regular);
    pub const LABEL: (f32, W) = (14.0, W::Medium);
    pub const SMALL: (f32, W) = (13.0, W::Regular);
    pub const SMALL_MEDIUM: (f32, W) = (13.0, W::Medium);
    pub const CAPTION: (f32, W) = (12.0, W::Medium);
    pub const SECTION: (f32, W) = (11.0, W::Bold);
}

pub fn f((size, w): (f32, W)) -> FontId {
    font(size, w)
}

// ------------------------------------------------------ spacing and shape

pub mod sp {
    pub const XXS: f32 = 2.0;
    pub const XS: f32 = 4.0;
    pub const S: f32 = 8.0;
    pub const M: f32 = 12.0;
    pub const L: f32 = 16.0;
    pub const XL: f32 = 24.0;
    pub const XXL: f32 = 32.0;
}

pub mod r {
    pub const BADGE: u8 = 4;
    pub const ROW: u8 = 6;
    pub const CONTROL: u8 = 8;
    pub const CARD: u8 = 12;
}

pub const SIDEBAR_W: f32 = 248.0;
pub const HEADER_H: f32 = 48.0;

pub fn shadow_overlay() -> Shadow {
    Shadow { offset: [0, 8], blur: 24, spread: 0, color: Color32::from_black_alpha(140) }
}

pub fn shadow_popup() -> Shadow {
    Shadow { offset: [0, 4], blur: 14, spread: 0, color: Color32::from_black_alpha(120) }
}

/// Hover/press colour transition time (DESIGN.md: 120 ms).
pub const ANIM: f32 = 0.12;

// ------------------------------------------------------------- install

pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);
    ctx.set_visuals(egui::Visuals::dark());
    ctx.global_style_mut(|s| {
        s.text_styles = [
            (egui::TextStyle::Small, f(ty::CAPTION)),
            (egui::TextStyle::Body, f(ty::LABEL)),
            (egui::TextStyle::Button, f(ty::LABEL)),
            (egui::TextStyle::Heading, f(ty::TITLE)),
            (egui::TextStyle::Monospace, FontId::monospace(13.0)),
        ]
        .into();

        let v = &mut s.visuals;
        v.panel_fill = c::BG_BASE;
        v.window_fill = c::BG_CONTENT;
        v.window_stroke = Stroke::NONE;
        v.window_corner_radius = CornerRadius::same(r::CARD);
        v.window_shadow = shadow_overlay();
        v.popup_shadow = shadow_popup();
        v.menu_corner_radius = CornerRadius::same(r::CONTROL);
        v.extreme_bg_color = c::BG_RAISED;
        v.text_edit_bg_color = Some(c::BG_RAISED);
        v.faint_bg_color = c::BG_HOVER_SOFT;
        v.code_bg_color = c::BG_RAISED;
        v.override_text_color = None;
        v.hyperlink_color = c::BRAND;
        v.warn_fg_color = c::WARNING;
        v.error_fg_color = c::DANGER;
        v.selection.bg_fill = alpha(c::BRAND, 0.45);
        v.selection.stroke = Stroke::new(1.0, c::BRAND);
        v.text_cursor.stroke = Stroke::new(2.0, c::TEXT);
        v.slider_trailing_fill = true;
        v.handle_shape = egui::style::HandleShape::Circle;
        v.indent_has_left_vline = false;

        let w = &mut v.widgets;
        let base = |wv: &mut egui::style::WidgetVisuals, bg: Color32, fg: Color32| {
            wv.bg_fill = bg;
            wv.weak_bg_fill = bg;
            wv.bg_stroke = Stroke::NONE;
            wv.fg_stroke = Stroke::new(1.5, fg);
            wv.corner_radius = CornerRadius::same(r::CONTROL);
            wv.expansion = 0.0;
        };
        base(&mut w.noninteractive, c::BG_CONTENT, c::TEXT_SECONDARY);
        w.noninteractive.bg_stroke = Stroke::new(1.0, c::BORDER);
        base(&mut w.inactive, c::BG_SELECTED, c::TEXT_SECONDARY);
        base(&mut w.hovered, c::BG_PRESSED, c::TEXT);
        base(&mut w.active, c::BG_PRESSED, c::TEXT);
        base(&mut w.open, c::BG_SELECTED, c::TEXT);
        w.hovered.bg_stroke = Stroke::new(1.0, c::BORDER_STRONG);
        w.active.bg_stroke = Stroke::new(1.0, c::BRAND);

        let sp = &mut s.spacing;
        sp.item_spacing = Vec2::new(sp::S, sp::S);
        sp.button_padding = Vec2::new(sp::M, 6.0);
        sp.interact_size = Vec2::new(32.0, 32.0);
        sp.combo_height = 280.0;
        sp.slider_rail_height = 6.0;
        sp.menu_margin = egui::Margin::same(6);
        sp.scroll = egui::style::ScrollStyle::thin();
        sp.scroll.bar_width = 6.0;
        sp.scroll.floating_allocated_width = 0.0;

        s.interaction.tooltip_delay = 0.25;
    });
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    // egui's bundled fonts (emoji etc.) stay as late fallbacks.
    let fallbacks = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();

    let mut add = |name: &str, bytes: &'static [u8]| {
        fonts.font_data.insert(name.into(), Arc::new(FontData::from_static(bytes)));
    };
    add("inter", include_bytes!("../../assets/fonts/Inter-Regular.ttf"));
    add("inter-medium", include_bytes!("../../assets/fonts/Inter-Medium.ttf"));
    add("inter-semibold", include_bytes!("../../assets/fonts/Inter-SemiBold.ttf"));
    add("inter-bold", include_bytes!("../../assets/fonts/Inter-Bold.ttf"));
    add("phosphor", egui_phosphor::Variant::Regular.font_bytes());
    add("phosphor-fill", egui_phosphor::Variant::Fill.font_bytes());

    let family = |primary: &str, icons: &str| {
        let mut v = vec![primary.to_string(), icons.to_string()];
        v.extend(fallbacks.iter().cloned());
        v
    };
    fonts.families.insert(FontFamily::Proportional, family("inter", "phosphor"));
    for (fam, file) in [
        ("medium", "inter-medium"),
        ("semibold", "inter-semibold"),
        ("bold", "inter-bold"),
    ] {
        fonts.families.insert(FontFamily::Name(fam.into()), family(file, "phosphor"));
    }
    // Icon families: icon glyphs first, Inter for anything else.
    fonts.families.insert(FontFamily::Name("icon".into()), family("phosphor", "inter"));
    fonts
        .families
        .insert(FontFamily::Name("icon-fill".into()), family("phosphor-fill", "inter"));
    ctx.set_fonts(fonts);
}
