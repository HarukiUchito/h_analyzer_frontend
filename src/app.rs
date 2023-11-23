//use egui_plotter::EguiBackend;
//use plotters::prelude::*;
use crate::common_data;
use crate::components::{dataframe_table, explorer, modal_window, plotter_2d};
use eframe::egui;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    organized: bool,

    #[serde(skip)]
    explorer: explorer::Explorer,

    common_data: common_data::CommonData,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            organized: false,
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
        if self.common_data.modal_window_open {
            if let Some(mut df_info) = self.common_data.dataframe_info.as_mut() {
                modal_window::ModalWindow::default().show(ctx, &mut df_info);
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.set_enabled(!self.common_data.modal_window_open);

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
            });
        });
        egui::SidePanel::left("info").show(ctx, |ui| {
            ui.set_enabled(!self.common_data.modal_window_open);

            ui.heading("h_analyzer");

            self.explorer.show(ui, &mut self.common_data);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_enabled(!self.common_data.modal_window_open);

            dataframe_table::DataFrameTable::default().show(ctx, &self.common_data);
            plotter_2d::Plotter2D::default().show(ctx, &self.common_data);

            if !self.organized {
                // organize windows
                ui.memory_mut(|mem| mem.reset_areas());
                self.organized = true;
            }
        });
    }
}
