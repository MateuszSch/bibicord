//! Profile page: display name and picture. In a room it replaces the
//! channel view; on the start screen it replaces the start card.

use super::theme::{self, c, f, sp, ty};
use super::widgets::{self as w, Avatar, Button, IconButton, Kind, ic};
use super::{App, Screen, View};
use crate::proto::{MAX_NAME, clean_name};
use eframe::egui::{self, Align2, CursorIcon, Key, Pos2, Rect, Sense, Stroke, TextureHandle, UiBuilder, Vec2};
use std::path::PathBuf;
use std::sync::mpsc;

/// Draft state while the page is open.
#[derive(Default)]
pub(crate) struct Edit {
    name: String,
    /// Where "close" returns to.
    back: Option<View>,
    /// Feedback under the form: (text, is error).
    note: Option<(String, bool)>,
    /// A picture being decoded and scaled off the UI thread.
    job: Option<mpsc::Receiver<Result<Vec<u8>, String>>>,
}

impl Edit {
    pub fn discard(&mut self) {
        self.note = None;
        self.job = None;
    }
}

/// Uploads a profile picture (PNG) as a texture. `None` if it does not decode.
pub(crate) fn texture(ctx: &egui::Context, key: &str, png: &[u8]) -> Option<TextureHandle> {
    let img = crate::avatar::decode(png)?;
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    // Mipmaps keep the picture smooth when drawn at 20 px.
    let opts = egui::TextureOptions { mipmap_mode: Some(egui::TextureFilter::Linear), ..egui::TextureOptions::LINEAR };
    Some(ctx.load_texture(format!("avatar {key}"), color, opts))
}

impl App {
    pub(crate) fn open_profile(&mut self) {
        if self.view != View::Profile {
            self.profile.back = Some(self.view);
        }
        self.profile.name = self.name.clone();
        self.profile.note = None;
        self.view = View::Profile;
    }

    pub(crate) fn close_profile(&mut self) {
        self.view = self.profile.back.take().unwrap_or(View::Chat);
        self.profile.discard();
    }

    /// Sets (or clears) the picture, tells the room, optionally saves it.
    pub(crate) fn set_avatar(&mut self, ctx: &egui::Context, png: Option<Vec<u8>>, persist: bool) {
        self.my_avatar_tex = png.as_deref().and_then(|p| texture(ctx, "me", p));
        self.my_avatar = png;
        self.node.set_profile(self.name.clone(), self.my_avatar.clone());
        if persist {
            if let Err(e) = crate::avatar::save(self.my_avatar.as_deref()) {
                self.profile.note = Some((format!("Nie udało się zapisać zdjęcia: {e:#}"), true));
            }
        }
    }

    fn save_name(&mut self) {
        let name = clean_name(&self.profile.name);
        if name.is_empty() {
            self.profile.note = Some(("Nazwa nie może być pusta.".into(), true));
            return;
        }
        self.profile.name = name.clone();
        if name == self.name {
            return;
        }
        self.name = name;
        let _ = std::fs::create_dir_all(crate::ffmpeg::data_dir());
        let _ = std::fs::write(super::name_file(), &self.name);
        self.node.set_profile(self.name.clone(), self.my_avatar.clone());
        self.profile.note = Some(("Zapisano nazwę.".into(), false));
    }

    fn load_picture(&mut self, ctx: &egui::Context, path: PathBuf) {
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(crate::avatar::prepare(&path).map_err(|e| format!("{e:#}")));
            ctx.request_repaint();
        });
        self.profile.job = Some(rx);
        self.profile.note = None;
    }

    fn poll_picture(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.profile.job else { return };
        match rx.try_recv() {
            Ok(Ok(png)) => {
                self.profile.job = None;
                self.profile.note = Some(("Zmieniono zdjęcie.".into(), false));
                self.set_avatar(ctx, Some(png), true);
            }
            Ok(Err(e)) => {
                self.profile.job = None;
                self.profile.note = Some((format!("Nie udało się użyć zdjęcia: {e}"), true));
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => self.profile.job = None,
        }
    }

    fn pick_picture(&mut self, ctx: &egui::Context) {
        let file = rfd::FileDialog::new()
            .set_title("Wybierz zdjęcie profilowe")
            .add_filter("Obrazy", &["jpg", "jpeg", "png", "webp"])
            .pick_file();
        if let Some(path) = file {
            self.load_picture(ctx, path);
        }
    }

    /// Full page inside the room (replaces chat or the voice stage).
    pub(crate) fn profile_page(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        // Header, in the style of the chat header.
        let full = ui.available_rect_before_wrap();
        let rect = Rect::from_min_size(full.min, Vec2::new(full.width(), theme::HEADER_H));
        ui.allocate_rect(rect, Sense::hover());
        ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, c::BG_BASE));
        let y = rect.center().y;
        w::paint_icon(ui, Pos2::new(rect.left() + sp::L + 10.0, y), ic::USER_CIRCLE, 22.0, c::TEXT_MUTED, false);
        let t = w::paint_text(ui, Pos2::new(rect.left() + sp::L + 28.0, y), Align2::LEFT_CENTER, "Profil", f(ty::HEADING), c::TEXT);
        let x = t.right() + sp::M;
        ui.painter().vline(x, (y - 10.0)..=(y + 10.0), Stroke::new(1.0, c::BORDER_STRONG));
        w::paint_text_clipped(
            ui,
            Pos2::new(x + sp::M, y),
            "Nazwę i zdjęcie widzą wszyscy w pokoju",
            f(ty::SMALL),
            c::TEXT_MUTED,
            rect.right() - x - 80.0,
        );
        let btn = Rect::from_center_size(rect.right_center() - Vec2::new(28.0, 0.0), Vec2::splat(32.0));
        let mut close = false;
        ui.scope_builder(UiBuilder::new().max_rect(btn), |ui| {
            close = IconButton::new(ic::X, "Zamknij (Esc)").show(ui).clicked();
        });
        let typing = ui.ctx().memory(|m| m.focused().is_some());
        close |= !typing && ui.input(|i| i.key_pressed(Key::Escape));

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let width = (ui.available_width() - 2.0 * sp::XXL).clamp(280.0, 520.0);
            ui.add_space(sp::XXL);
            ui.horizontal(|ui| {
                ui.add_space(sp::XXL);
                ui.vertical(|ui| {
                    ui.set_width(width);
                    ui.spacing_mut().item_spacing = Vec2::new(sp::S, 0.0);
                    self.profile_form(ui, width);
                });
            });
            ui.add_space(sp::XXL);
        });
        if close {
            self.close_profile();
        }
    }

    /// Start screen variant: the card shows the form with a back button.
    pub(crate) fn profile_card(&mut self, ui: &mut egui::Ui) {
        let width = ui.available_width();
        let mut back = false;
        ui.horizontal(|ui| {
            back = IconButton::new(ic::ARROW_LEFT, "Wstecz").show(ui).clicked();
            ui.add_space(sp::XS);
            ui.label(w::text("Profil", ty::TITLE, c::TEXT));
        });
        ui.add_space(sp::XS);
        ui.label(w::text("Nazwę i zdjęcie zobaczą znajomi w pokoju.", ty::SMALL, c::TEXT_MUTED));
        ui.add_space(sp::XL);
        self.profile_form(ui, width);
        back |= ui.input(|i| i.key_pressed(Key::Escape)) && ui.ctx().memory(|m| m.focused().is_none());
        if back {
            self.close_profile();
        }
    }

    fn profile_form(&mut self, ui: &mut egui::Ui, width: f32) {
        let ctx = ui.ctx().clone();
        self.poll_picture(&ctx);
        let loading = self.profile.job.is_some();
        if !loading {
            let dropped = ctx.input(|i| i.raw.dropped_files.iter().next().map(|f| f.path().to_path_buf()));
            if let Some(path) = dropped {
                self.load_picture(&ctx, path);
            }
        }
        let loading = self.profile.job.is_some();
        let files_over = ctx.input(|i| !i.raw.hovered_files.is_empty());

        // Picture
        w::field_label(ui, "Zdjęcie profilowe");
        ui.add_space(sp::XS);
        let mut pick = false;
        let mut remove = false;
        ui.horizontal(|ui| {
            let side = 104.0;
            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(side), Sense::click());
            let resp = resp.on_hover_text("Zmień zdjęcie").on_hover_cursor(CursorIcon::PointingHand);
            let t = w::hover_t(ui, &resp);
            let radius = side / 2.0 - 4.0;
            let shown = clean_name(&self.profile.name);
            Avatar {
                name: if shown.is_empty() { &self.name } else { &shown },
                color: self.my_color(),
                image: self.my_avatar_tex.as_ref(),
                radius,
                speaking: 0.0,
                status: None,
            }
            .paint(ui, rect.center());
            let p = ui.painter();
            if loading {
                p.circle_filled(rect.center(), radius, theme::alpha(c::BG_STAGE, 0.6));
                egui::Spinner::new()
                    .size(28.0)
                    .color(c::TEXT)
                    .paint_at(ui, Rect::from_center_size(rect.center(), Vec2::splat(28.0)));
            } else if t > 0.0 || files_over {
                let a = if files_over { 1.0 } else { t };
                p.circle_filled(rect.center(), radius, theme::alpha(c::BG_STAGE, 0.55 * a));
                w::paint_icon(ui, rect.center(), ic::CAMERA, 28.0, theme::alpha(c::TEXT, a), false);
            }
            if files_over {
                p.circle_stroke(rect.center(), radius + 3.0, Stroke::new(2.0, c::BRAND));
            }
            pick |= resp.clicked();

            ui.add_space(sp::L);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(sp::S, sp::S);
                ui.add_space(sp::S);
                ui.horizontal(|ui| {
                    let label = if self.my_avatar.is_some() { "Zmień zdjęcie" } else { "Wybierz zdjęcie" };
                    pick |= Button::new(label, Kind::Secondary)
                        .icon(ic::IMAGE_SQUARE)
                        .height(34.0)
                        .enabled(!loading)
                        .show(ui)
                        .clicked();
                    if self.my_avatar.is_some() {
                        remove = Button::new("Usuń", Kind::Ghost)
                            .icon(ic::TRASH)
                            .height(34.0)
                            .enabled(!loading)
                            .show(ui)
                            .clicked();
                    }
                });
                ui.add(
                    egui::Label::new(w::text(
                        "JPG, PNG lub WebP. Możesz też przeciągnąć plik do okna. \
                         Zdjęcie zostanie przycięte do kwadratu.",
                        ty::SMALL,
                        c::TEXT_MUTED,
                    ))
                    .wrap(),
                );
            });
        });
        if pick && !loading {
            self.pick_picture(&ctx);
        }
        if remove {
            self.set_avatar(&ctx, None, true);
            self.profile.note = Some(("Usunięto zdjęcie.".into(), false));
        }

        // Name
        ui.add_space(sp::XL);
        w::field_label(ui, "Nazwa wyświetlana");
        let save_w = 108.0;
        let mut save = false;
        ui.horizontal(|ui| {
            let resp = w::text_field(ui, &mut self.profile.name, "np. Marcin", "profile_name", width - save_w - sp::S);
            if self.profile.name.chars().count() > MAX_NAME {
                self.profile.name = self.profile.name.chars().take(MAX_NAME).collect();
            }
            if resp.changed() {
                self.profile.note = None;
            }
            let draft = clean_name(&self.profile.name);
            let can = !draft.is_empty() && draft != self.name;
            save |= can && resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            save |= Button::new("Zapisz", Kind::Primary)
                .icon(ic::CHECK)
                .width(save_w)
                .height(42.0)
                .enabled(can)
                .show(ui)
                .clicked();
        });
        if save {
            self.save_name();
        }
        ui.add_space(sp::XS);
        let count = self.profile.name.trim().chars().count();
        let hint = if self.screen == Screen::Room {
            format!("Zmiana od razu widoczna dla wszystkich w pokoju · {count}/{MAX_NAME}")
        } else {
            format!("Maks. {MAX_NAME} znaki · {count}/{MAX_NAME}")
        };
        ui.label(w::text(hint, ty::CAPTION, c::TEXT_MUTED));

        if let Some((text, err)) = &self.profile.note {
            ui.add_space(sp::M);
            let (icon, col) = if *err { (ic::WARNING_CIRCLE, c::DANGER) } else { (ic::CHECK_CIRCLE, c::SUCCESS) };
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(sp::XS + 2.0, 0.0);
                ui.label(w::icon(icon, 16.0, col));
                ui.label(w::text(text, ty::SMALL, col));
            });
        }
    }
}
