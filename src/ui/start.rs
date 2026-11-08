//! First screen: name, create a room or join with an invite.

use super::theme::{self, W, c, f, r, sp, ty};
use super::widgets::{self as w, Button, Kind, ic};
use super::App;
use eframe::egui::{self, Align2, Key, Pos2, Sense, Vec2};

const CARD_W: f32 = 440.0;

impl App {
    pub(crate) fn start_screen(&mut self, root: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(c::BG_BASE))
            .show(root, |ui| {
                let avail = ui.available_rect_before_wrap();
                let card_h = 560.0;
                let top = (avail.height() - card_h).max(24.0) / 2.0;
                ui.add_space(top);
                ui.vertical_centered(|ui| {
                    egui::Frame::new()
                        .fill(c::BG_CONTENT)
                        .corner_radius(r::CARD)
                        .shadow(theme::shadow_overlay())
                        .inner_margin(egui::Margin::same(sp::XXL as i8))
                        .show(ui, |ui| {
                            ui.set_width(CARD_W - 2.0 * sp::XXL);
                            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                self.start_card(ui)
                            });
                        });
                });
            });
    }

    fn start_card(&mut self, ui: &mut egui::Ui) {
        let width = ui.available_width();
        // Mark + heading
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
            ui.painter().rect_filled(rect, r::CARD, c::BRAND);
            w::paint_icon(ui, rect.center(), ic::WAVEFORM, 26.0, c::WHITE, true);
            ui.add_space(sp::XS);
            ui.vertical(|ui| {
                ui.add_space(1.0);
                ui.label(w::text("Witaj w BibiCord", ty::DISPLAY, c::TEXT));
                ui.label(w::text(
                    "Głos, czat i streamy w jakości źródła. Bez serwera.",
                    ty::SMALL,
                    c::TEXT_MUTED,
                ));
            });
        });
        ui.add_space(sp::XL);

        w::field_label(ui, "Twoja nazwa");
        w::text_field(ui, &mut self.name, "np. Marcin", "start_name", width);
        ui.add_space(sp::L);

        w::field_label(ui, "Nazwa pokoju");
        let default_room = if self.name.trim().is_empty() {
            "Pokój".to_string()
        } else {
            format!("Pokój {}", self.name.trim())
        };
        w::text_field(ui, &mut self.room_input, &default_room, "start_room", width);
        ui.add_space(sp::M);

        let can = self.net_ready && !self.name.trim().is_empty();
        if Button::new("Utwórz pokój", Kind::Primary)
            .icon(ic::PLUS)
            .width(width)
            .height(42.0)
            .enabled(can)
            .show(ui)
            .clicked()
        {
            self.name = self.name.trim().to_string();
            let room = match self.room_input.trim() {
                "" => default_room,
                r => r.to_string(),
            };
            self.node.create_room(self.name.clone(), room);
            self.enter_room();
        }

        // "lub" divider
        ui.add_space(sp::L);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 16.0), Sense::hover());
        let y = rect.center().y;
        let label = ui.painter().layout_no_wrap("lub".into(), f(ty::CAPTION), c::TEXT_MUTED);
        let lw = label.size().x + 24.0;
        ui.painter().hline(rect.left()..=rect.center().x - lw / 2.0, y, (1.0, c::BORDER));
        ui.painter().hline(rect.center().x + lw / 2.0..=rect.right(), y, (1.0, c::BORDER));
        ui.painter().galley(rect.center() - label.size() / 2.0, label, c::TEXT_MUTED);
        ui.add_space(sp::L);

        w::field_label(ui, "Zaproszenie");
        let join_w = 96.0;
        let mut join = false;
        ui.horizontal(|ui| {
            let resp = w::text_field(
                ui,
                &mut self.ticket_input,
                "np. K7QX-3MFA-9TZP",
                "start_ticket",
                width - join_w - sp::S,
            );
            if resp.changed() {
                self.join_error = None;
            }
            join |= resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            let can_join = can && !self.ticket_input.trim().is_empty();
            join &= can_join;
            join |= Button::new("Dołącz", Kind::Secondary)
                .width(join_w)
                .height(42.0)
                .enabled(can_join)
                .show(ui)
                .clicked();
        });
        if let Some(err) = &self.join_error {
            ui.add_space(sp::XS);
            ui.label(w::text(err, ty::SMALL, c::DANGER));
        }
        if join {
            self.name = self.name.trim().to_string();
            match self.node.join(&self.ticket_input, self.name.clone()) {
                Ok(()) => {
                    self.ticket_input.clear();
                    self.join_error = None;
                    self.enter_room();
                }
                Err(e) => self.join_error = Some(format!("{e:#}. Sprawdź, czy skopiowano cały kod.")),
            }
        }

        // Footer: status
        ui.add_space(sp::XL);
        w::separator(ui, c::BORDER);
        ui.add_space(sp::M);
        let ff = self.ffmpeg_status.lock().unwrap().clone();
        let (dot, line) = if !self.net_ready {
            (c::WARNING, "Łączenie z siecią…".to_string())
        } else if !ff.is_empty() {
            (c::WARNING, ff)
        } else if self.name.trim().is_empty() {
            (c::SUCCESS, "Sieć gotowa. Wpisz swoją nazwę.".to_string())
        } else {
            (c::SUCCESS, "Sieć gotowa".to_string())
        };
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(8.0, 16.0), Sense::hover());
            ui.painter().circle_filled(rect.center(), 4.0, dot);
            ui.label(w::text(line, ty::SMALL, c::TEXT_MUTED));
        });
        let _ = (Align2::LEFT_TOP, Pos2::ZERO, W::Regular);
    }
}
