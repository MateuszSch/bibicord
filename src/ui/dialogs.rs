//! Modal dialogs (Go Live, Settings, Invite) and the error toast.

use super::theme::{self, W, c, f, r, sp, ty};
use super::widgets::{self as w, Button, IconButton, Kind, ic};
use super::{App, Dialog};
use crate::proto::Codec;
use eframe::egui::{self, Align, Align2, Layout, Margin, Stroke, Ui, Vec2};
use std::time::{Duration, Instant};

const DIALOG_W: f32 = 440.0;

fn res_label(h: u32) -> String {
    match h {
        0 => "Źródło (natywna)".into(),
        2160 => "2160p (4K)".into(),
        h => format!("{h}p"),
    }
}

/// Title row with a close button. Returns true when close was clicked.
fn header(ui: &mut Ui, title: &str, subtitle: &str) -> bool {
    let mut close = false;
    egui::Frame::new()
        .inner_margin(Margin { left: 24, right: 16, top: 20, bottom: 4 })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(w::text(title, ty::TITLE, c::TEXT));
                    if !subtitle.is_empty() {
                        ui.add_space(sp::XXS);
                        ui.label(w::text(subtitle, ty::SMALL, c::TEXT_MUTED));
                    }
                });
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if IconButton::new(ic::X, "Zamknij").size(28.0).show(ui).clicked() {
                        close = true;
                    }
                });
            });
        });
    close
}

fn body(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .inner_margin(Margin { left: 24, right: 24, top: 12, bottom: 20 })
        .show(ui, add);
}

/// Footer strip on a darker surface, buttons aligned right.
fn footer(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .fill(c::BG_SIDEBAR)
        .corner_radius(egui::CornerRadius { nw: 0, ne: 0, sw: r::CARD, se: r::CARD })
        .inner_margin(Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.with_layout(Layout::right_to_left(Align::Center), add);
        });
}

fn combo_style(ui: &mut Ui) {
    let v = &mut ui.style_mut().visuals.widgets;
    for wv in [&mut v.inactive, &mut v.hovered, &mut v.active, &mut v.open] {
        wv.bg_fill = c::BG_RAISED;
        wv.weak_bg_fill = c::BG_RAISED;
    }
    v.hovered.bg_stroke = Stroke::new(1.0, c::BORDER_STRONG);
    ui.style_mut().spacing.button_padding = Vec2::new(12.0, 9.0);
}

/// Device picker: `None` means the system default.
fn device_combo(ui: &mut Ui, id: &str, value: &mut Option<String>, list: &[crate::voice::DeviceInfo]) -> bool {
    let before = value.clone();
    let selected = value
        .as_ref()
        .and_then(|v| list.iter().find(|d| &d.id == v))
        .map(|d| d.name.clone())
        .unwrap_or_else(|| "Domyślne systemowe".into());
    combo_style(ui);
    egui::ComboBox::from_id_salt(id)
        .width(ui.available_width())
        .selected_text(w::text(selected, ty::BODY, c::TEXT))
        .show_ui(ui, |ui| {
            ui.selectable_value(value, None, w::text("Domyślne systemowe", ty::SMALL_MEDIUM, c::TEXT));
            for d in list {
                ui.selectable_value(value, Some(d.id.clone()), w::text(&d.name, ty::SMALL_MEDIUM, c::TEXT));
            }
        });
    *value != before
}

impl App {
    pub(crate) fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(d) = self.dialog else { return };
        let modal = egui::Modal::new(egui::Id::unique(("dialog", d as u8)))
            .frame(w::modal_frame())
            .backdrop_color(egui::Color32::from_black_alpha(170))
            .show(ctx, |ui| {
                ui.set_width(DIALOG_W);
                ui.spacing_mut().item_spacing = Vec2::new(sp::S, sp::S);
                match d {
                    Dialog::GoLive => self.go_live_ui(ui),
                    Dialog::Settings => self.settings_ui(ui),
                    Dialog::Invite => self.invite_ui(ui),
                }
            });
        if modal.should_close() {
            self.dialog = None;
        }
    }

    fn go_live_ui(&mut self, ui: &mut Ui) {
        if header(ui, "Udostępnij ekran", "Strumień w jakości źródła, prosto do znajomych.") {
            self.dialog = None;
        }
        body(ui, |ui| {
            let s = &mut self.settings;
            w::field_label(ui, "Ekran");
            if self.displays.is_empty() {
                ui.label(w::text("Nie znaleziono ekranów.", ty::SMALL, c::TEXT_MUTED));
            } else {
                // Display cards: one per screen.
                ui.horizontal_wrapped(|ui| {
                    for (i, name) in self.displays.iter().enumerate() {
                        let sel = s.display == i;
                        let size = Vec2::new(((DIALOG_W - 48.0 - sp::S) / 2.0).floor(), 64.0);
                        let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
                        let t = w::hover_t(ui, &resp);
                        let bg = if sel { theme::alpha(c::BRAND, 0.14) } else { theme::mix(c::BG_RAISED, c::BG_SELECTED, t) };
                        ui.painter().rect_filled(rect, r::CONTROL, bg);
                        if sel {
                            ui.painter().rect_stroke(rect, r::CONTROL, Stroke::new(1.5, c::BRAND), egui::StrokeKind::Inside);
                        }
                        let fg = if sel { c::TEXT } else { c::TEXT_SECONDARY };
                        w::paint_icon(ui, rect.left_center() + Vec2::new(26.0, 0.0), ic::MONITOR, 24.0, fg, sel);
                        let title = format!("Ekran {}", i + 1);
                        let has_sub = name.trim() != title;
                        let dy = if has_sub { -8.0 } else { 0.0 };
                        w::paint_text_clipped(
                            ui,
                            rect.left_center() + Vec2::new(48.0, dy),
                            &title,
                            f(ty::LABEL),
                            fg,
                            rect.width() - 60.0,
                        );
                        if has_sub {
                            w::paint_text_clipped(
                                ui,
                                rect.left_center() + Vec2::new(48.0, 10.0),
                                name,
                                f(ty::CAPTION),
                                c::TEXT_MUTED,
                                rect.width() - 60.0,
                            );
                        }
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            s.display = i;
                        }
                    }
                });
            }

            ui.add_space(sp::M);
            w::field_label(ui, "Rozdzielczość");
            ui.horizontal_wrapped(|ui| {
                for h in [0, 2160, 1440, 1080, 720] {
                    let label = if h == 0 { "Źródło".to_string() } else { format!("{h}p") };
                    if w::chip(ui, &label, s.max_height == h).on_hover_text(res_label(h)).clicked() {
                        s.max_height = h;
                    }
                }
            });

            ui.add_space(sp::M);
            ui.columns(2, |cols| {
                w::field_label(&mut cols[0], "Klatki na sekundę");
                cols[0].horizontal(|ui| {
                    for fps in [30, 60, 120] {
                        if w::chip(ui, &fps.to_string(), s.fps == fps).clicked() {
                            s.fps = fps;
                        }
                    }
                });
                w::field_label(&mut cols[1], "Kodek");
                cols[1].horizontal(|ui| {
                    for (codec, label) in [(Codec::H264, "H.264"), (Codec::Hevc, "HEVC")] {
                        if w::chip(ui, label, s.codec == codec).clicked() {
                            s.codec = codec;
                        }
                    }
                });
            });

            ui.add_space(sp::M);
            w::field_label_value(ui, "Bitrate", &format!("{} Mb/s", s.bitrate_kbps / 1000));
            ui.spacing_mut().slider_width = ui.available_width();
            ui.add(
                egui::Slider::new(&mut s.bitrate_kbps, 4_000..=120_000)
                    .step_by(1000.0)
                    .show_value(false),
            );

            ui.add_space(sp::M);
            egui::Frame::new()
                .fill(theme::alpha(c::BRAND, 0.08))
                .corner_radius(r::CONTROL)
                .inner_margin(Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        ui.label(w::icon(ic::INFO, 16.0, c::BRAND));
                        ui.add(
                            egui::Label::new(w::text(
                                "Obraz jest kodowany sprzętowo raz i trafia do widzów bez \
                                 transkodowania. Każdy widz pobiera pełny bitrate z Twojego łącza.",
                                ty::SMALL,
                                c::TEXT_SECONDARY,
                            ))
                            .wrap(),
                        );
                    });
                });
        });
        footer(ui, |ui| {
            let ok = Button::new("Rozpocznij stream", Kind::Primary)
                .icon(ic::BROADCAST)
                .enabled(!self.displays.is_empty())
                .show(ui);
            if ok.clicked() {
                self.dialog = None;
                self.stream_starting = true;
                self.node.start_stream(self.settings.clone());
            }
            if Button::new("Anuluj", Kind::Ghost).show(ui).clicked() {
                self.dialog = None;
            }
        });
    }

    fn settings_ui(&mut self, ui: &mut Ui) {
        if header(ui, "Ustawienia głosu", "Urządzenia audio używane w rozmowie.") {
            self.dialog = None;
        }
        let (inputs, outputs) = self.audio_devices.clone().unwrap_or_default();
        let mut dev = self.node.devices();
        let mut changed = false;
        body(ui, |ui| {
            w::field_label(ui, "Mikrofon");
            changed |= device_combo(ui, "dev_in", &mut dev.input, &inputs);
            ui.add_space(sp::M);
            w::field_label(ui, "Głośniki / słuchawki");
            changed |= device_combo(ui, "dev_out", &mut dev.output, &outputs);
            ui.add_space(sp::L);
            w::separator(ui, c::BORDER);
            ui.add_space(sp::M);
            w::field_label(ui, "Twój profil");
            ui.horizontal(|ui| {
                w::Avatar { name: &self.name, color: self.my_color(), radius: 18.0, speaking: 0.0, status: None }.show(ui);
                ui.vertical(|ui| {
                    ui.label(w::text(&self.name, ty::HEADING, c::TEXT));
                    ui.label(w::text(
                        format!("Głos: Opus 48 kHz · v{}", env!("CARGO_PKG_VERSION")),
                        ty::CAPTION,
                        c::TEXT_MUTED,
                    ));
                });
            });
        });
        if changed {
            self.node.set_devices(dev);
        }
        footer(ui, |ui| {
            if Button::new("Gotowe", Kind::Primary).show(ui).clicked() {
                self.dialog = None;
            }
        });
    }

    fn invite_ui(&mut self, ui: &mut Ui) {
        let title = format!("Zaproś do: {}", self.room);
        if header(ui, &title, "Wyślij kod znajomemu. Połączenie jest bezpośrednie, bez serwera.") {
            self.dialog = None;
        }
        let ctx = ui.ctx().clone();
        body(ui, |ui| {
            w::field_label(ui, "Kod zaproszenia");
            let copied = self.copied_at.is_some_and(|t| t.elapsed() < Duration::from_secs(2));
            if copied {
                ctx.request_repaint_after(Duration::from_millis(250));
            }
            egui::Frame::new()
                .fill(c::BG_BASE)
                .corner_radius(r::CONTROL)
                .inner_margin(Margin { left: 12, right: 6, top: 6, bottom: 6 })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let avail = ui.available_width() - 104.0;
                        let shown = self.ticket.as_deref().unwrap_or("Generowanie…");
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(avail, 38.0), egui::Sense::hover());
                        w::paint_text_clipped(
                            ui,
                            rect.left_center(),
                            shown,
                            egui::FontId::monospace(13.0),
                            c::TEXT_BODY,
                            avail,
                        );
                        let (label, kind, icon) = if copied {
                            ("Skopiowano", Kind::Success, ic::CHECK)
                        } else {
                            ("Kopiuj", Kind::Primary, ic::COPY)
                        };
                        let b = Button::new(label, kind).icon(icon).width(100.0).enabled(self.ticket.is_some()).show(ui);
                        if b.clicked() {
                            if let Some(t) = &self.ticket {
                                ctx.copy_text(t.clone());
                                self.copied_at = Some(Instant::now());
                            }
                        }
                    });
                });
            ui.add_space(sp::L);
            for (n, step) in [
                "Skopiuj kod i wyślij go znajomemu.",
                "Znajomy wkleja kod na ekranie startowym i klika Dołącz.",
                "Pokój działa, dopóki ktoś w nim jest.",
            ]
            .iter()
            .enumerate()
            {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(22.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 11.0, c::BG_SELECTED);
                    w::paint_text(ui, rect.center(), Align2::CENTER_CENTER, &(n + 1).to_string(), theme::font(11.0, W::Bold), c::TEXT_SECONDARY);
                    ui.label(w::text(*step, ty::SMALL, c::TEXT_SECONDARY));
                });
            }
        });
    }

    pub(crate) fn toast(&mut self, ctx: &egui::Context) {
        let Some((msg, at)) = &self.error else { return };
        if at.elapsed() > Duration::from_secs(7) {
            self.error = None;
            return;
        }
        ctx.request_repaint_after(Duration::from_millis(500));
        let msg = msg.clone();
        let mut close = false;
        egui::Area::new(egui::Id::unique("toast"))
            .order(egui::Order::Tooltip)
            .anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -24.0))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(c::BG_RAISED)
                    .stroke(Stroke::new(1.0, theme::alpha(c::DANGER, 0.5)))
                    .corner_radius(r::CONTROL)
                    .shadow(theme::shadow_popup())
                    .inner_margin(Margin { left: 12, right: 6, top: 8, bottom: 8 })
                    .show(ui, |ui| {
                        ui.set_max_width(520.0);
                        ui.horizontal(|ui| {
                            ui.label(w::icon(ic::WARNING_CIRCLE, 20.0, c::DANGER));
                            ui.add(egui::Label::new(w::text(&msg, ty::SMALL_MEDIUM, c::TEXT)).wrap());
                            if IconButton::new(ic::X, "Zamknij").size(26.0).show(ui).clicked() {
                                close = true;
                            }
                        });
                    });
            });
        if close {
            self.error = None;
        }
    }
}
