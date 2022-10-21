//hide windows console
//#![windows_subsystem = "windows"]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use eframe::egui::{self, Grid, ScrollArea};

use librusl::{
    manager::{FinalResults, Manager, SearchResult},
    search::Search,
};

pub fn main() {
    let native_options = eframe::NativeOptions {
        icon_data: Some(load_icon()),
        ..Default::default()
    };

    eframe::run_native("rusl", native_options, Box::new(|cc| Box::new(AppState::new(cc))));
}
fn load_icon() -> eframe::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(include_bytes!("icons/icon.png")).unwrap().into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    eframe::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}

pub struct AppState {
    search_name: String,
    search_content: String,
    show_settings: bool,
    results: Arc<Mutex<FinalResults>>,
    manager: Manager,
    message: String,
    last_id: usize,
}

impl eframe::App for AppState {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.manager.save();
        // eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.menu(frame, ctx);
        self.side_panel(ctx);
        self.central_panel(ctx);
    }
}

impl AppState {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // if let Some(storage) = cc.storage {
        //     // return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        // }

        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        let results = Arc::new(Mutex::new(FinalResults {
            data: vec![],
            duration: Duration::from_secs(0),
            id: 0,
        }));
        let results_thread = results.clone();

        let (sx, rx) = std::sync::mpsc::channel();
        let manager = Manager::new(sx);
        spawn_receiver(rx, results_thread, cc.egui_ctx.clone());

        Self {
            // Example stuff:
            search_name: "".to_string(),
            search_content: "".to_string(),

            show_settings: false,
            results,
            manager,
            message: "Ready to search".to_string(),
            last_id: 0,
        }
    }

    fn menu(&mut self, frame: &mut eframe::Frame, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        self.manager.save_and_quit();
                        frame.close();
                    }
                });
            });
        });
    }
    fn side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("side_panel").min_width(200.).show(ctx, |ui| {
            ui.label("Search");

            ui.label("File Name");
            let sn = ui.text_edit_singleline(&mut self.search_name);
            if sn.lost_focus() && sn.ctx.input().key_pressed(egui::Key::Enter) {
                self.do_search()
            }

            ui.label("");
            ui.label("File contents");
            let se = ui.text_edit_singleline(&mut self.search_content);
            if se.lost_focus() && se.ctx.input().key_pressed(egui::Key::Enter) {
                self.do_search()
            }

            ui.label("");
            ui.label("Directory");
            ui.horizontal(|ui| {
                if ui.button("📁").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        let mut ops = self.manager.get_options();
                        ops.last_dir = folder.to_string_lossy().to_string();
                        self.manager.set_options(ops);
                    }
                }
                ui.text_edit_singleline(&mut self.manager.get_options().last_dir);
            });
            ui.add_space(10.);

            if ui.button("Find").clicked() {
                self.do_search();
            }
            ui.add_space(10.);
            ui.label(&self.message);
            ui.add_space(40.0);
            if ui.button("Clipboard").on_hover_text("Save to clipboard").clicked() {
                self.manager
                    .export(self.results.lock().unwrap().data.iter().map(|x| x.path.to_string()).collect());
            }
            if ui
                .button((if self.show_settings { "Show results" } else { "Show settings" }).to_string())
                .clicked()
            {
                self.show_settings = !self.show_settings;
            }
            ui.add_space(40.);
            ui.separator();
            ui.hyperlink("https://github.com/griccardos/rusl");
        });
    }

    fn do_search(&mut self) {
        if self.search_name.is_empty() && self.search_content.is_empty() {
            self.message = "Nothing to search for".to_string();
        } else if !PathBuf::from(&self.manager.get_options().last_dir).exists() {
            self.message = "Invalid directory".to_string();
        } else {
            self.manager.search(Search {
                name_text: self.search_name.clone(),
                contents_text: self.search_content.clone(),
                dir: self.manager.get_options().last_dir,
            });
            self.message = "Searching...".to_string();
        }
    }
    fn central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.show_settings {
                self.settings_panel(ui);
            } else {
                self.results_panel(ui);
            }
        });
    }

    fn settings_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.label("Name settings");
        ui.checkbox(&mut self.manager.get_options().name.case_sensitive, "Case sensitive");
        ui.checkbox(&mut self.manager.get_options().name.same_filesystem, "Same filesystem");
        ui.checkbox(&mut self.manager.get_options().name.ignore_dot, "Ignore dot files");
        ui.checkbox(&mut self.manager.get_options().name.follow_links, "Follow links");
        ui.label("Contents");
        ui.checkbox(&mut self.manager.get_options().content.case_sensitive, "Case sensitive");
    }

    fn results_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Results");
        ScrollArea::new([true, true]).min_scrolled_height(200.).show(ui, |ui| {
            Grid::new("grid").num_columns(2).striped(true).show(ui, |ui| {
                if let Ok(results) = self.results.try_lock() {
                    for r in results.data.iter().take(2000) {
                        ui.label(&r.path);
                        const MAX_COUNT: usize = 100;
                        const MAX_LEN: usize = 200;
                        let mut content = r.content(MAX_COUNT, MAX_LEN);
                        if r.matches.len() > MAX_COUNT {
                            content.push_str(&format!("\nand {} other lines", r.matches.len() - MAX_COUNT));
                        }
                        if !content.is_empty() {
                            ui.label(&content);
                        }
                        ui.end_row();
                    }
                    if results.data.len() > 2000 {
                        ui.label(format!("and {} others...", results.data.len() - 2000));
                    }
                    if results.id != self.last_id {
                        self.last_id = results.id;
                        self.message = format!("Found {} results in {:.2}s", results.data.len(), results.duration.as_secs_f64());
                    }
                }
            });
        });
    }
}

fn spawn_receiver(rx: std::sync::mpsc::Receiver<SearchResult>, results_thread: Arc<Mutex<FinalResults>>, context: egui::Context) {
    thread::spawn(move || loop {
        if let Ok(rec) = rx.recv() {
            match rec {
                SearchResult::FinalResults(res) => {
                    let mut res_self = results_thread.lock().unwrap();
                    if res.id > res_self.id {
                        res_self.data = res.data;
                        res_self.duration = res.duration;
                        res_self.id = res.id;
                        context.request_repaint();
                    }
                }
                SearchResult::InterimResult(_) => {
                    //TODO. currently only final results are loaded.
                    //this is to show interim results as they are received
                }
            };
        } else {
            println!("error recv");
        }
    });
}
