//! Minimal UI (egui).

use crate::net::{Event, Events, Node};
use crate::proto::{Codec, StreamInfo};
use crate::video::encode::{self, Settings};
use eframe::egui::{
    self, Align, Color32, CornerRadius, FontId, Key, Layout, RichText, Sense, Stroke, TextEdit,
    TextureHandle, TextureOptions, Vec2, ViewportCommand,
};
use iroh::EndpointId;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const BG: Color32 = Color32::from_rgb(0x0f, 0x10, 0x12);
const PANEL: Color32 = Color32::from_rgb(0x16, 0x17, 0x1a);
const RAISED: Color32 = Color32::from_rgb(0x20, 0x22, 0x26);
const TEXT: Color32 = Color32::from_rgb(0xe4, 0xe5, 0xe8);
const MUTED: Color32 = Color32::from_rgb(0x82, 0x86, 0x8e);
const ACCENT: Color32 = Color32::from_rgb(0x6d, 0x8c, 0xff);
const LIVE: Color32 = Color32::from_rgb(0xf0, 0x4a, 0x5a);
const SPEAK: Color32 = Color32::from_rgb(0x3d, 0xd6, 0x8c);

struct PeerView {
    id: EndpointId,
    name: String,
    stream: Option<StreamInfo>,
    voice: bool,
}

struct ChatLine {
    name: String,
    mine: bool,
    text: String,
}

#[derive(PartialEq)]
enum Screen {
    Start,
    Room,
}

pub struct App {
    node: Node,
    events: std::sync::mpsc::Receiver<Event>,
    screen: Screen,
    net_ready: bool,
    ffmpeg_status: Arc<Mutex<String>>,

    name: String,
    ticket_input: String,
    ticket: Option<String>,
    copied_at: Option<Instant>,

    peers: Vec<PeerView>,
    chat: Vec<ChatLine>,
    chat_input: String,

    my_stream: Option<StreamInfo>,
    stream_starting: bool,
    stream_dialog: bool,
    settings: Settings,
    displays: Vec<String>,

    my_voice: bool,
    voice_starting: bool,
    muted: bool,
    deafened: bool,

    watching: Option<EndpointId>,
    texture: Option<TextureHandle>,
    video_size: [usize; 2],
    fps: (Instant, u64, f32),
    fullscreen: bool,

    status: String,
    error: Option<(String, Instant)>,
    demo: Option<crate::demo::Demo>,
}

fn name_file() -> std::path::PathBuf {
    crate::ffmpeg::data_dir().join("name.txt")
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, rt: tokio::runtime::Handle) -> Self {
        style(&cc.egui_ctx);
        let (tx, rx) = std::sync::mpsc::channel();
        let events = Events::new(tx, cc.egui_ctx.clone());
        let node = Node::new(rt, events);

        let ffmpeg_status = Arc::new(Mutex::new("Przygotowywanie FFmpeg…".to_string()));
        {
            let st = ffmpeg_status.clone();
            let ctx = cc.egui_ctx.clone();
            std::thread::spawn(move || {
                let set = |s: String| {
                    *st.lock().unwrap() = s;
                    ctx.request_repaint();
                };
                match crate::ffmpeg::ensure(&set) {
                    Ok(_) => set(String::new()),
                    Err(e) => set(format!("FFmpeg niedostępny: {e:#}")),
                }
            });
        }

        let name = std::fs::read_to_string(name_file())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        App {
            node,
            events: rx,
            screen: Screen::Start,
            net_ready: false,
            ffmpeg_status,
            name,
            ticket_input: String::new(),
            ticket: None,
            copied_at: None,
            peers: Vec::new(),
            chat: Vec::new(),
            chat_input: String::new(),
            my_stream: None,
            stream_starting: false,
            stream_dialog: false,
            settings: Settings::default(),
            displays: Vec::new(),
            my_voice: false,
            voice_starting: false,
            muted: false,
            deafened: false,
            watching: None,
            texture: None,
            video_size: [0, 0],
            fps: (Instant::now(), 0, 0.0),
            fullscreen: false,
            status: String::new(),
            error: None,
            demo: crate::demo::Demo::from_env(),
        }
    }

    fn handle_events(&mut self) {
        while let Ok(ev) = self.events.try_recv() {
            match ev {
                Event::Ready => self.net_ready = true,
                Event::Ticket(t) => self.ticket = Some(t),
                Event::PeerJoined { id, name } => {
                    if let Some(p) = self.peers.iter_mut().find(|p| p.id == id) {
                        p.name = name;
                    } else {
                        self.status.clear();
                        self.peers.push(PeerView { id, name, stream: None, voice: false });
                    }
                }
                Event::PeerLeft { id } => self.peers.retain(|p| p.id != id),
                Event::PeerStream { id, info } => {
                    if let Some(p) = self.peers.iter_mut().find(|p| p.id == id) {
                        p.stream = info;
                    }
                }
                Event::PeerVoice { id, on } => {
                    if let Some(p) = self.peers.iter_mut().find(|p| p.id == id) {
                        p.voice = on;
                    }
                }
                Event::Chat { from, name, text } => {
                    self.chat.push(ChatLine { name, mine: from.is_none(), text })
                }
                Event::MyStream(info) => {
                    self.stream_starting = false;
                    self.my_stream = info;
                }
                Event::MyVoice(on) => {
                    self.voice_starting = false;
                    self.my_voice = on;
                    if on {
                        self.muted = false;
                        self.deafened = false;
                    }
                }
                Event::Watching(w) => {
                    self.watching = w;
                    if w.is_none() {
                        self.texture = None;
                    }
                }
                Event::Status(s) => self.status = s,
                Event::Error(e) => {
                    self.stream_starting = false;
                    self.voice_starting = false;
                    self.status.clear();
                    self.error = Some((e, Instant::now()));
                }
            }
        }
    }

    fn enter_room(&mut self) {
        let _ = std::fs::create_dir_all(crate::ffmpeg::data_dir());
        let _ = std::fs::write(name_file(), &self.name);
        self.screen = Screen::Room;
    }

    fn leave_room(&mut self, ctx: &egui::Context) {
        self.node.leave();
        self.screen = Screen::Start;
        self.peers.clear();
        self.chat.clear();
        self.ticket = None;
        self.my_stream = None;
        self.my_voice = false;
        self.watching = None;
        self.texture = None;
        self.status.clear();
        if self.fullscreen {
            self.fullscreen = false;
            ctx.send_viewport_cmd(ViewportCommand::Fullscreen(false));
        }
    }

    // ------------------------------------------------------------ start

    fn start_screen(&mut self, root: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG))
            .show(root, |ui| {
                let w = 320.0;
                ui.add_space((ui.available_height() - 340.0).max(20.0) / 2.0);
                ui.vertical_centered(|ui| {
                    ui.set_max_width(w);
                    ui.label(RichText::new("bibicord").size(30.0).strong().color(TEXT));
                    ui.label(RichText::new("bez serwera · jakość źródła").color(MUTED));
                    ui.add_space(28.0);

                    ui.add(
                        TextEdit::singleline(&mut self.name)
                            .hint_text("Twoje imię")
                            .desired_width(w)
                            .margin(Vec2::new(10.0, 8.0)),
                    );
                    ui.add_space(10.0);
                    let can = self.net_ready && !self.name.trim().is_empty();
                    if ui.add_enabled(can, primary("Nowy pokój", w)).clicked() {
                        self.name = self.name.trim().to_string();
                        self.node.create_room(self.name.clone());
                        self.enter_room();
                    }
                    ui.add_space(18.0);
                    ui.label(RichText::new("lub dołącz").color(MUTED).size(12.0));
                    ui.add_space(6.0);
                    let resp = ui.add(
                        TextEdit::singleline(&mut self.ticket_input)
                            .hint_text("Wklej zaproszenie")
                            .desired_width(w)
                            .margin(Vec2::new(10.0, 8.0)),
                    );
                    ui.add_space(8.0);
                    let can_join = can && !self.ticket_input.trim().is_empty();
                    let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    if ui.add_enabled(can_join, secondary("Dołącz", w)).clicked() || (enter && can_join) {
                        self.name = self.name.trim().to_string();
                        match self.node.join(&self.ticket_input, self.name.clone()) {
                            Ok(()) => {
                                self.ticket_input.clear();
                                self.enter_room();
                            }
                            Err(e) => self.error = Some((format!("{e:#}"), Instant::now())),
                        }
                    }
                    ui.add_space(24.0);
                    let net = if self.net_ready { "sieć gotowa" } else { "łączenie z siecią…" };
                    ui.label(RichText::new(net).color(MUTED).size(12.0));
                    let ff = self.ffmpeg_status.lock().unwrap().clone();
                    if !ff.is_empty() {
                        ui.label(RichText::new(ff).color(MUTED).size(12.0));
                    }
                });
            });
    }

    // ------------------------------------------------------------- room

    fn sidebar(&mut self, root: &mut egui::Ui) {
        let speaking: Vec<EndpointId> = self.node.voice().map(|v| v.speaking()).unwrap_or_default();
        let me_speaking = self.node.voice().is_some_and(|v| v.me_speaking());

        egui::Panel::left("side")
            .exact_size(236.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(14.0))
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("bibicord").strong().size(16.0).color(TEXT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let copied = self.copied_at.is_some_and(|t| t.elapsed() < Duration::from_secs(2));
                        let label = if copied { "skopiowano" } else { "zaproś" };
                        let enabled = self.ticket.is_some();
                        if ui
                            .add_enabled(enabled, egui::Button::new(RichText::new(label).size(12.0)).small())
                            .on_hover_text("Skopiuj zaproszenie do schowka")
                            .clicked()
                        {
                            if let Some(t) = &self.ticket {
                                ui.ctx().copy_text(t.clone());
                                self.copied_at = Some(Instant::now());
                            }
                        }
                    });
                });
                ui.add_space(16.0);
                ui.label(RichText::new("W POKOJU").size(11.0).color(MUTED));
                ui.add_space(6.0);

                // Me
                member_row(
                    ui,
                    &format!("{} (ty)", self.name),
                    self.my_voice,
                    me_speaking,
                    self.my_stream.is_some(),
                    false,
                    None,
                );
                let mut watch_click = None;
                for p in &self.peers {
                    let path = self.node.path_kind(&p.id);
                    let watching = self.watching == Some(p.id);
                    if member_row(
                        ui,
                        &p.name,
                        p.voice,
                        speaking.contains(&p.id),
                        p.stream.is_some(),
                        watching,
                        path,
                    ) {
                        watch_click = Some(p.id);
                    }
                }
                if let Some(id) = watch_click {
                    if self.watching == Some(id) {
                        self.node.unwatch();
                    } else {
                        self.node.watch(id);
                    }
                }
                if self.peers.is_empty() {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Wyślij komuś zaproszenie, żeby dołączył.")
                            .color(MUTED)
                            .size(12.0),
                    );
                }

                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    let w = ui.available_width();
                    if ui.add(ghost("Wyjdź", w)).clicked() {
                        self.leave_room(ui.ctx());
                    }
                    ui.add_space(6.0);
                    if let Some(info) = &self.my_stream {
                        let n = self.node.viewer_count();
                        ui.label(
                            RichText::new(format!("{} · widzowie: {n}", info.summary()))
                                .size(11.0)
                                .color(MUTED),
                        );
                        if ui.add(danger("Zakończ stream", w)).clicked() {
                            self.node.stop_stream();
                        }
                    } else {
                        let ff_ready = crate::ffmpeg::path().is_some();
                        let label = if self.stream_starting { "Uruchamianie…" } else { "Udostępnij ekran" };
                        if ui
                            .add_enabled(ff_ready && !self.stream_starting, secondary(label, w))
                            .clicked()
                        {
                            self.displays = encode::displays();
                            if self.settings.display >= self.displays.len() {
                                self.settings.display = 0;
                            }
                            self.stream_dialog = true;
                        }
                    }
                    ui.add_space(6.0);
                    if self.my_voice {
                        ui.horizontal(|ui| {
                            let half = (w - ui.spacing().item_spacing.x) / 2.0;
                            let m = if self.muted { "Włącz mikrofon" } else { "Wycisz" };
                            if ui.add(toggle(m, half, self.muted)).clicked() {
                                self.muted = !self.muted;
                                if let Some(v) = self.node.voice() {
                                    v.set_muted(self.muted);
                                }
                            }
                            let d = if self.deafened { "Słuchaj" } else { "Ogłusz" };
                            if ui.add(toggle(d, half, self.deafened)).clicked() {
                                self.deafened = !self.deafened;
                                if let Some(v) = self.node.voice() {
                                    v.set_deafened(self.deafened);
                                }
                            }
                        });
                        if ui.add(ghost("Rozłącz głos", w)).clicked() {
                            self.node.leave_voice();
                        }
                    } else {
                        let label = if self.voice_starting { "Łączenie…" } else { "Dołącz do głosu" };
                        if ui.add_enabled(!self.voice_starting, primary(label, w)).clicked() {
                            self.voice_starting = true;
                            self.node.join_voice();
                        }
                    }
                    if !self.status.is_empty() {
                        ui.add_space(6.0);
                        ui.label(RichText::new(&self.status).size(12.0).color(MUTED));
                    }
                });
            });
    }

    fn chat_panel(&mut self, ui: &mut egui::Ui) {
        let input_h = 44.0;
        let avail = ui.available_height() - input_h;
        egui::ScrollArea::vertical()
            .max_height(avail.max(50.0))
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.chat.is_empty() {
                    ui.add_space(8.0);
                    ui.label(RichText::new("Brak wiadomości.").color(MUTED));
                }
                let mut prev: Option<&str> = None;
                for line in &self.chat {
                    if prev != Some(line.name.as_str()) {
                        ui.add_space(10.0);
                        let c = if line.mine { ACCENT } else { TEXT };
                        ui.label(RichText::new(&line.name).strong().color(c));
                    }
                    ui.label(RichText::new(&line.text).color(TEXT));
                    prev = Some(line.name.as_str());
                }
            });
        ui.add_space(6.0);
        let resp = ui.add(
            TextEdit::singleline(&mut self.chat_input)
                .hint_text("Napisz wiadomość…")
                .desired_width(f32::INFINITY)
                .margin(Vec2::new(10.0, 9.0)),
        );
        if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            let text = self.chat_input.trim().to_string();
            if !text.is_empty() {
                self.node.send_chat(text);
            }
            self.chat_input.clear();
            resp.request_focus();
        }
    }

    fn video(&mut self, ui: &mut egui::Ui) {
        let slot = self.node.frames();
        if let Some(f) = slot.take() {
            let img = egui::ColorImage::from_rgba_premultiplied([f.width, f.height], &f.rgba);
            self.video_size = [f.width, f.height];
            match &mut self.texture {
                Some(t) => t.set(img, TextureOptions::LINEAR),
                None => {
                    self.texture = Some(ui.ctx().load_texture("video", img, TextureOptions::LINEAR))
                }
            }
        }
        let n = slot.count();
        if self.fps.0.elapsed() >= Duration::from_secs(1) {
            let dt = self.fps.0.elapsed().as_secs_f32();
            self.fps = (Instant::now(), n, (n - self.fps.1) as f32 / dt);
        }

        let rect = ui.available_rect_before_wrap();
        let resp = ui.allocate_rect(rect, Sense::click());
        ui.painter().rect_filled(rect, 0.0, Color32::BLACK);
        if let Some(t) = &self.texture {
            let [w, h] = self.video_size;
            let scale = (rect.width() / w as f32).min(rect.height() / h as f32);
            let size = Vec2::new(w as f32, h as f32) * scale;
            let r = egui::Rect::from_center_size(rect.center(), size);
            ui.painter().image(
                t.id(),
                r,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Łączenie ze streamem…",
                FontId::proportional(14.0),
                MUTED,
            );
        }

        // Overlay
        let who = self
            .watching
            .and_then(|id| self.peers.iter().find(|p| p.id == id))
            .map(|p| p.name.clone())
            .unwrap_or_default();
        let hovered = resp.hovered() || resp.contains_pointer();
        if hovered || !self.fullscreen {
            let txt = if self.video_size[0] > 0 {
                format!(
                    "{who} · {}×{} · {:.0} fps",
                    self.video_size[0], self.video_size[1], self.fps.2
                )
            } else {
                who
            };
            let galley = ui.painter().layout_no_wrap(txt, FontId::proportional(12.0), TEXT);
            let pos = rect.left_top() + Vec2::new(12.0, 12.0);
            let bg = egui::Rect::from_min_size(pos - Vec2::splat(6.0), galley.size() + Vec2::splat(12.0));
            ui.painter().rect_filled(bg, 6.0, Color32::from_black_alpha(160));
            ui.painter().galley(pos, galley, TEXT);

            let close = egui::Rect::from_min_size(rect.right_top() + Vec2::new(-40.0, 10.0), Vec2::splat(28.0));
            let full = egui::Rect::from_min_size(rect.right_top() + Vec2::new(-74.0, 10.0), Vec2::splat(28.0));
            if overlay_button(ui, close, Icon::Close).clicked() {
                self.node.unwatch();
            }
            let icon = if self.fullscreen { Icon::Shrink } else { Icon::Expand };
            if overlay_button(ui, full, icon).clicked() {
                self.toggle_fullscreen(ui.ctx());
            }
        }
        if resp.double_clicked() {
            self.toggle_fullscreen(ui.ctx());
        }
    }

    fn run_demo(&mut self, ctx: &egui::Context) {
        let Some(mut demo) = self.demo.take() else { return };
        let d = &mut demo;
        ctx.request_repaint_after(Duration::from_millis(200));
        if self.screen == Screen::Start && self.net_ready {
            if let Some(n) = d.name.take() {
                self.name = n;
            }
            if d.create.is_some() {
                self.node.create_room(self.name.clone());
                self.enter_room();
            } else if let Some(f) = &d.join {
                if let Ok(t) = std::fs::read_to_string(f) {
                    if self.node.join(&t, self.name.clone()).is_ok() {
                        self.enter_room();
                    }
                }
            }
        }
        if let (Some(f), Some(t), false) = (&d.create, &self.ticket, d.ticket_written) {
            let _ = std::fs::write(f, t);
            d.ticket_written = true;
        }
        if !d.chat_sent && !self.peers.is_empty() {
            d.chat_sent = true;
            for line in d.chat.drain(..) {
                self.node.send_chat(line);
            }
        }
        if d.stream && !d.stream_started && !self.peers.is_empty() && crate::ffmpeg::path().is_some() {
            d.stream_started = true;
            self.node.start_stream(self.settings.clone());
        }
        if d.watch && self.watching.is_none() {
            if let Some(p) = self.peers.iter().find(|p| p.stream.is_some()) {
                self.node.watch(p.id);
            }
        }
        if !d.shot_requested && Instant::now() >= d.shot_at {
            if let Some(path) = d.shot.clone() {
                d.shot_requested = true;
                let c = ctx.clone();
                ctx.send_viewport_cmd(ViewportCommand::Screenshot(egui::ScreenshotCallback::new(
                    move |img| {
                        crate::demo::save_png(&path, &img);
                        c.send_viewport_cmd(ViewportCommand::Close);
                        c.request_repaint();
                    },
                )));
            }
        }
        self.demo = Some(demo);
    }

    fn toggle_fullscreen(&mut self, ctx: &egui::Context) {
        self.fullscreen = !self.fullscreen;
        ctx.send_viewport_cmd(ViewportCommand::Fullscreen(self.fullscreen));
    }

    fn stream_dialog(&mut self, ctx: &egui::Context) {
        if !self.stream_dialog {
            return;
        }
        let modal = egui::Modal::new(egui::Id::unique("stream")).show(ctx, |ui| {
            ui.set_width(320.0);
            ui.label(RichText::new("Udostępnij ekran").strong().size(16.0));
            ui.add_space(12.0);
            let s = &mut self.settings;
            egui::Grid::new("opts").num_columns(2).spacing([16.0, 10.0]).show(ui, |ui| {
                ui.label(RichText::new("Ekran").color(MUTED));
                egui::ComboBox::from_id_salt("disp")
                    .width(200.0)
                    .selected_text(self.displays.get(s.display).cloned().unwrap_or_default())
                    .show_ui(ui, |ui| {
                        for (i, d) in self.displays.iter().enumerate() {
                            ui.selectable_value(&mut s.display, i, d);
                        }
                    });
                ui.end_row();

                ui.label(RichText::new("Rozdzielczość").color(MUTED));
                egui::ComboBox::from_id_salt("res")
                    .width(200.0)
                    .selected_text(res_label(s.max_height))
                    .show_ui(ui, |ui| {
                        for h in [0, 2160, 1440, 1080, 720] {
                            ui.selectable_value(&mut s.max_height, h, res_label(h));
                        }
                    });
                ui.end_row();

                ui.label(RichText::new("Klatki").color(MUTED));
                ui.horizontal(|ui| {
                    for f in [30, 60, 120] {
                        ui.selectable_value(&mut s.fps, f, format!("{f} fps"));
                    }
                });
                ui.end_row();

                ui.label(RichText::new("Bitrate").color(MUTED));
                ui.add(
                    egui::Slider::new(&mut s.bitrate_kbps, 4_000..=120_000)
                        .step_by(1000.0)
                        .custom_formatter(|v, _| format!("{:.0} Mb/s", v / 1000.0)),
                );
                ui.end_row();

                ui.label(RichText::new("Kodek").color(MUTED));
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut s.codec, Codec::H264, "H.264");
                    ui.selectable_value(&mut s.codec, Codec::Hevc, "HEVC");
                });
                ui.end_row();
            });
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "Obraz jest kodowany sprzętowo raz i trafia do widzów bez transkodowania. \
                     Każdy widz pobiera pełny bitrate z Twojego łącza.",
                )
                .size(11.0)
                .color(MUTED),
            );
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                if ui.add(ghost("Anuluj", 120.0)).clicked() {
                    self.stream_dialog = false;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(primary("Start", 120.0)).clicked() {
                        self.stream_dialog = false;
                        self.stream_starting = true;
                        self.node.start_stream(self.settings.clone());
                    }
                });
            });
        });
        if modal.should_close() {
            self.stream_dialog = false;
        }
    }

    fn toast(&mut self, ctx: &egui::Context) {
        let Some((msg, at)) = &self.error else { return };
        if at.elapsed() > Duration::from_secs(7) {
            self.error = None;
            return;
        }
        egui::Area::new(egui::Id::unique("toast"))
            .anchor(egui::Align2::CENTER_BOTTOM, Vec2::new(0.0, -20.0))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(Color32::from_rgb(0x3a, 0x1d, 0x22))
                    .stroke(Stroke::new(1.0, LIVE))
                    .corner_radius(8.0)
                    .inner_margin(Vec2::new(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.set_max_width(520.0);
                        ui.label(RichText::new(msg).color(TEXT));
                    });
            });
        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_events();
        self.run_demo(&ctx);
        if self.fullscreen && ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.toggle_fullscreen(&ctx);
        }
        if self.fullscreen && self.watching.is_none() {
            self.toggle_fullscreen(&ctx);
        }
        match self.screen {
            Screen::Start => self.start_screen(ui),
            Screen::Room => {
                if self.watching.is_some() {
                    if !self.fullscreen {
                        self.sidebar(ui);
                        egui::Panel::right("chat")
                            .default_size(300.0)
                            .min_size(220.0)
                            .frame(egui::Frame::new().fill(PANEL).inner_margin(14.0))
                            .show(ui, |ui| self.chat_panel(ui));
                    }
                    egui::CentralPanel::default()
                        .frame(egui::Frame::new().fill(Color32::BLACK))
                        .show(ui, |ui| self.video(ui));
                } else {
                    self.sidebar(ui);
                    egui::CentralPanel::default()
                        .frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::symmetric(24, 16)))
                        .show(ui, |ui| self.chat_panel(ui));
                }
                self.stream_dialog(&ctx);
                if self.my_voice || self.my_stream.is_some() {
                    ctx.request_repaint_after(Duration::from_millis(150));
                } else if !self.peers.is_empty() {
                    ctx.request_repaint_after(Duration::from_secs(1));
                }
            }
        }
        if !self.ffmpeg_status.lock().unwrap().is_empty() {
            ctx.request_repaint_after(Duration::from_millis(500));
        }
        self.toast(&ctx);
    }

    fn on_exit(&mut self) {
        self.node.shutdown();
    }
}

fn res_label(h: u32) -> String {
    match h {
        0 => "Źródło (natywna)".into(),
        2160 => "4K (2160p)".into(),
        h => format!("{h}p"),
    }
}

/// One member row. Returns true if the LIVE badge was clicked.
fn member_row(
    ui: &mut egui::Ui,
    name: &str,
    voice: bool,
    speaking: bool,
    live: bool,
    watching: bool,
    path: Option<&str>,
) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.set_min_height(26.0);
        let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
        let dot = if speaking {
            SPEAK
        } else if voice {
            Color32::from_rgb(0x4a, 0x50, 0x5a)
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().circle_filled(r.center(), 4.0, dot);
        if speaking {
            ui.painter().circle_stroke(r.center(), 6.0, Stroke::new(1.0, SPEAK));
        }
        ui.label(RichText::new(name).color(TEXT));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if live {
                let (fill, text) = if watching { (ACCENT, "OGLĄDASZ") } else { (LIVE, "LIVE") };
                let b = egui::Button::new(RichText::new(text).size(10.0).strong().color(Color32::WHITE))
                    .fill(fill)
                    .corner_radius(4.0)
                    .small();
                if path.is_some() {
                    clicked = ui.add(b).on_hover_text("Oglądaj").clicked();
                } else {
                    ui.add_enabled(false, b);
                }
            }
            if let Some(p) = path {
                ui.label(RichText::new(p).size(10.0).color(MUTED));
            }
        });
    });
    clicked
}

#[derive(Clone, Copy, Debug, Hash)]
enum Icon {
    Close,
    Expand,
    Shrink,
}

fn overlay_button(ui: &mut egui::Ui, rect: egui::Rect, icon: Icon) -> egui::Response {
    let resp = ui.interact(rect, ui.scope_id().with(icon), Sense::click());
    let a = if resp.hovered() { 210 } else { 140 };
    let p = ui.painter();
    p.rect_filled(rect, 6.0, Color32::from_black_alpha(a));
    let st = Stroke::new(1.6, TEXT);
    let c = rect.center();
    let r = 6.0;
    match icon {
        Icon::Close => {
            p.line_segment([c + Vec2::new(-r, -r), c + Vec2::new(r, r)], st);
            p.line_segment([c + Vec2::new(-r, r), c + Vec2::new(r, -r)], st);
        }
        Icon::Expand | Icon::Shrink => {
            let l = 3.5;
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let corner = c + Vec2::new(sx * r, sy * r);
                // Expand: brackets point outwards; Shrink: inwards.
                let (hx, vy) = if matches!(icon, Icon::Expand) {
                    (Vec2::new(-sx * l, 0.0), Vec2::new(0.0, -sy * l))
                } else {
                    (Vec2::new(sx * l, 0.0), Vec2::new(0.0, sy * l))
                };
                p.line_segment([corner, corner + hx], st);
                p.line_segment([corner, corner + vy], st);
            }
        }
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn btn(label: &str, w: f32, fill: Color32, text: Color32) -> egui::Button<'static> {
    egui::Button::new(RichText::new(label.to_string()).color(text))
        .fill(fill)
        .min_size(Vec2::new(w, 34.0))
        .corner_radius(8.0)
}

fn primary(label: &str, w: f32) -> egui::Button<'static> {
    btn(label, w, ACCENT, Color32::WHITE)
}

fn secondary(label: &str, w: f32) -> egui::Button<'static> {
    btn(label, w, RAISED, TEXT)
}

fn danger(label: &str, w: f32) -> egui::Button<'static> {
    btn(label, w, LIVE, Color32::WHITE)
}

fn ghost(label: &str, w: f32) -> egui::Button<'static> {
    btn(label, w, Color32::TRANSPARENT, MUTED).stroke(Stroke::new(1.0, RAISED))
}

fn toggle(label: &str, w: f32, on: bool) -> egui::Button<'static> {
    if on { btn(label, w, LIVE, Color32::WHITE) } else { secondary(label, w) }
}

fn style(ctx: &egui::Context) {
    ctx.set_visuals(egui::Visuals::dark());
    ctx.global_style_mut(|s| {
        let v = &mut s.visuals;
        v.panel_fill = BG;
        v.window_fill = PANEL;
        v.extreme_bg_color = RAISED;
        v.override_text_color = Some(TEXT);
        v.selection.bg_fill = ACCENT.linear_multiply(0.6);
        v.hyperlink_color = ACCENT;
        v.window_corner_radius = CornerRadius::same(12);
        v.window_stroke = Stroke::new(1.0, RAISED);
        for w in [
            &mut v.widgets.noninteractive,
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            w.corner_radius = CornerRadius::same(8);
        }
        v.widgets.inactive.weak_bg_fill = RAISED;
        v.widgets.inactive.bg_fill = RAISED;
        v.widgets.inactive.bg_stroke = Stroke::NONE;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT.linear_multiply(0.5));
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, RAISED);
        s.spacing.item_spacing = Vec2::new(8.0, 6.0);
        s.spacing.button_padding = Vec2::new(10.0, 5.0);
    });
}
