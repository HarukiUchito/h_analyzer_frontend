#[derive(serde::Deserialize, serde::Serialize, PartialEq)]
enum DataFrameType {
    NDEV,
    KITTI,
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq)]
enum ExplorerTab {
    FILESYSTEM,
    DATAFRAME,
}

use std::collections::HashMap;

//use egui_plotter::EguiBackend;
//use plotters::prelude::*;
use eframe::{egui, emath::Align2};
use polars::prelude::*;
use poll_promise::Promise;

use crate::backend_talk;
use crate::backend_talk::grpc_fs;
use crate::components::{dataframe_table, plotter_2d};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    backend: backend_talk::BackendTalk,

    organized: bool,

    modal_window_open: bool,
    dataframe_type: Option<DataFrameType>,
    filepath_to_be_loaded: Option<String>,

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
            dataframe_type: None,
            filepath_to_be_loaded: None,

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

        if self.modal_window_open {
            egui::Window::new("modal")
                //                .open(&mut self.modal_window_open)
                .anchor(Align2::CENTER_TOP, egui::Vec2::new(0.0, 100.0))
                .show(ctx, |ui| {
                    let dfname = if let Some(fpath) = self.filepath_to_be_loaded.as_ref() {
                        let pathv = std::path::Path::new(fpath);
                        let name = pathv.file_name().unwrap().to_string_lossy().to_string();
                        ui.label(name.clone());
                        name
                    } else {
                        "null".to_string()
                    };
                    ui.label("dataframe type");
                    if ui
                        .add(egui::RadioButton::new(
                            if let Some(dtype) = &self.dataframe_type {
                                dtype == &DataFrameType::NDEV
                            } else {
                                false
                            },
                            "NDEV",
                        ))
                        .clicked()
                    {
                        self.dataframe_type = Some(DataFrameType::NDEV);
                    }
                    if ui
                        .add(egui::RadioButton::new(
                            if let Some(dtype) = &self.dataframe_type {
                                dtype == &DataFrameType::KITTI
                            } else {
                                false
                            },
                            "KITTI",
                        ))
                        .clicked()
                    {
                        self.dataframe_type = Some(DataFrameType::KITTI);
                    }
                    if ui.button("Load File").clicked() {
                        if let Some(filepath) = &self.filepath_to_be_loaded {
                            self.hello_promise =
                                Some((dfname, self.backend.load_df_request(filepath.clone())));
                            self.filepath_to_be_loaded = None;
                            self.modal_window_open = false;
                        }
                    }
                });
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
                                            self.filepath_to_be_loaded =
                                                Some(nfp.to_string_lossy().to_string());
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
