use std::time::Duration;

//use egui_plotter::EguiBackend;
//use plotters::prelude::*;
use crate::common_data;
use crate::components::{dataframe_table, explorer, modal_window, plotter_2d};
use eframe::egui::{self, FontData};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    organized: bool,

    #[serde(skip)]
    explorer: explorer::Explorer,
    #[serde(skip)]
    dataframe_table: dataframe_table::DataFrameTable,
    #[serde(skip)]
    plotter_2d: plotter_2d::Plotter2D,

    #[serde(skip)]
    common_data: common_data::CommonData,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            organized: false,
            dataframe_table: dataframe_table::DataFrameTable::default(),
            plotter_2d: plotter_2d::Plotter2D::default(),
            explorer: explorer::Explorer::default(),
            common_data: common_data::CommonData::default(),
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        let mut fonts = egui::FontDefinitions::default();

        // Install my own font (maybe supporting non-latin characters):
        fonts.font_data.insert(
            "my_font".to_owned(),
            FontData::from_static(include_bytes!("../NotoSansJP-Regular.ttf")),
        ); // .ttf and .otf supported
           // Put my font first (highest priority) for proportional text:
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "my_font".to_owned());

        // Put my font as last fallback for monospace:
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("my_font".to_owned());

        cc.egui_ctx.set_fonts(fonts);

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let current_path = self.common_data.current_path.clone();
        self.common_data.current_path = self.common_data.default_path.clone();
        eframe::set_value(storage, eframe::APP_KEY, self);
        self.common_data.current_path = current_path;
        log::info!("saved state");
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        //
        // Data update
        //
        self.common_data.update();

        //
        // View update
        //
        let mut opening_modal_window = false;
        for (df_info, _) in self.common_data.dataframes.iter_mut() {
            if df_info.load_state == modal_window::LoadState::OPEN_MODAL_WINDOW {
                modal_window::ModalWindow::default().show(ctx, df_info);
                opening_modal_window = true;
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.set_enabled(!opening_modal_window);

            egui::menu::bar(ui, |ui| {
                #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
                {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            _frame.close();
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_dark_light_mode_buttons(ui);

                if ui.button("reset layout").clicked() {
                    self.organized = false;
                }

                if ui.button("reset dataframes").clicked() {
                    self.common_data.dataframes.clear();
                }

                if ui.button("save").clicked() {
                    self.save(_frame.storage_mut().unwrap());
                }
            });
        });
        egui::SidePanel::left("info").show(ctx, |ui| {
            ui.set_enabled(!opening_modal_window);

            ui.heading("h_analyzer");

            self.explorer.show(ui, &mut self.common_data);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_enabled(!opening_modal_window);

            self.dataframe_table.show(ctx, &self.common_data);
            self.plotter_2d.show(ctx, &self.common_data);

            if !self.organized {
                // organize windows
                ui.memory_mut(|mem| mem.reset_areas());
                self.organized = true;
            }
        });

        egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            ui.label("Progress");
        });
    }
}
