#[derive(serde::Deserialize, serde::Serialize, PartialEq)]
enum ExplorerTab {
    FILESYSTEM,
    DATAFRAME,
}

use std::borrow::BorrowMut;
use std::collections::HashMap;

//use egui_plotter::EguiBackend;
//use plotters::prelude::*;
use eframe::egui;
use polars::prelude::*;
use poll_promise::Promise;

use crate::backend_talk;
use crate::backend_talk::grpc_fs;
use crate::components::{dataframe_table, modal_window, plotter_2d};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    backend: backend_talk::BackendTalk,

    organized: bool,

    modal_window_open: bool,
    dataframe_info: Option<modal_window::DataFrameInfo>,

    explorer_tab: ExplorerTab,
    dataframes: HashMap<String, DataFrame>,

    #[serde(skip)]
    hello_promise: Option<(String, Promise<Result<DataFrame, tonic::Status>>)>,

    current_path: String,
    default_path: String,
    #[serde(skip)]
    d_path_promise: Option<Promise<Result<grpc_fs::PathMessage, tonic::Status>>>,
    #[serde(skip)]
    fs_list_promise: Option<Promise<Result<grpc_fs::ListResponse, tonic::Status>>>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let backend = backend_talk::BackendTalk::default();

        let path = "/".to_string();
        let d_path_promise = backend.request_default_path();
        let fs_list_promise = backend.request_list(path.clone());
        Self {
            backend: backend,
            organized: false,
            modal_window_open: false,

            dataframe_info: None,

            explorer_tab: ExplorerTab::FILESYSTEM,
            dataframes: HashMap::new(),

            hello_promise: None,
            current_path: path.clone(),
            default_path: path.clone(),
            d_path_promise: Some(d_path_promise),
            fs_list_promise: Some(fs_list_promise),
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
        let current_path = self.current_path.clone();
        self.current_path = self.default_path.clone();
        eframe::set_value(storage, eframe::APP_KEY, self);
        self.current_path = current_path;
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        //
        // Data update
        //
        if let Some(d_path_promise) = &self.d_path_promise {
            if let Some(d_path) = d_path_promise.ready() {
                log::info!("d_path_promise: {:?}", d_path);
                let d_path = d_path.as_ref().unwrap().path.clone();
                self.current_path = d_path.clone();
                self.default_path = d_path.clone();
                self.fs_list_promise = Some(self.backend.request_list(d_path.clone()));
                self.d_path_promise = None;
            }
        }

        let df = (|| {
            if let Some((dname, result)) = &self.hello_promise {
                if let Some(result) = result.ready() {
                    if let Ok(result) = result {
                        self.dataframes.insert(dname.clone(), result.clone());
                        return result.clone();
                    }
                }
            }
            DataFrame::default()
        })();

        if let Some(df_info) = self.dataframe_info.as_mut() {
            if df_info.load_now {
                let name = modal_window::get_filename(df_info.filepath.as_str());
                self.hello_promise =
                    Some((name, self.backend.load_df_request(df_info.filepath.clone())));
                self.modal_window_open = false;
                df_info.load_now = false;
            }
        }

        //
        // View update
        //
        if self.modal_window_open {
            if let Some(mut df_info) = self.dataframe_info.as_mut() {
                modal_window::ModalWindow::default().show(ctx, &mut df_info);
            }
        }
        // Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.set_enabled(!self.modal_window_open);
            // The top panel is often a good place for a menu bar:

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
            ui.set_enabled(!self.modal_window_open);

            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("h_analyzer");

            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.explorer_tab, ExplorerTab::FILESYSTEM, "Files");
                ui.selectable_value(&mut self.explorer_tab, ExplorerTab::DATAFRAME, "DataFrames");
            });

            ui.separator();

            match self.explorer_tab {
                ExplorerTab::FILESYSTEM => {
                    ui.label(self.current_path.as_str());
                    if ui.button("refresh").clicked() {
                        self.fs_list_promise =
                            Some(self.backend.request_list(self.current_path.clone()));
                    }

                    ui.separator();

                    egui::ScrollArea::both().show(ui, |ui| {
                        let mut update_list = false;
                        if let Some(fs_list) = &self.fs_list_promise {
                            if let Some(fs_list) = fs_list.ready() {
                                if let Ok(fs_list) = fs_list {
                                    let mut fsvec = fs_list.files.clone();
                                    fsvec.sort();
                                    let mut b1 = false;
                                    let dirname = "..";
                                    if ui.checkbox(&mut b1, dirname).double_clicked() {
                                        let nfp = std::path::Path::new(self.current_path.as_str())
                                            .join(dirname);
                                        self.current_path = nfp.to_string_lossy().into_owned();
                                        update_list = true;
                                    }
                                    for dirname in fs_list.directories.iter() {
                                        if ui.checkbox(&mut b1, dirname).double_clicked() {
                                            let nfp =
                                                std::path::Path::new(self.current_path.as_str())
                                                    .join(dirname);
                                            self.current_path = nfp.to_string_lossy().into_owned();
                                            update_list = true;
                                        }
                                    }
                                    for filename in fsvec.iter() {
                                        let mut b1 = false;
                                        if ui.checkbox(&mut b1, filename).double_clicked() {
                                            self.modal_window_open = true;
                                            let nfp =
                                                std::path::Path::new(self.current_path.as_str())
                                                    .join(filename);
                                            self.dataframe_info =
                                                Some(modal_window::DataFrameInfo::new(
                                                    nfp.to_string_lossy().to_string(),
                                                ));
                                        }
                                    }
                                }
                            }
                        }
                        if update_list {
                            self.fs_list_promise =
                                Some(self.backend.request_list(self.current_path.to_string()));
                        }
                    });
                }
                ExplorerTab::DATAFRAME => {
                    for (name, _) in self.dataframes.iter() {
                        let mut checkd = false;
                        ui.checkbox(&mut checkd, name.clone());
                    }
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_enabled(!self.modal_window_open);

            dataframe_table::DataFrameTable::default().show(ctx, &df);
            plotter_2d::Plotter2D::default().show(ctx, &df);

            if !self.organized {
                // organize windows
                ui.memory_mut(|mem| mem.reset_areas());
                self.organized = true;
            }
        });
    }
}
