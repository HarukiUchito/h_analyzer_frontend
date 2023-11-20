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
use eframe::egui;
use polars::prelude::*;
use poll_promise::Promise;

use crate::backend_talk;
use crate::backend_talk::grpc_fs;

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

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum Enum {
    First,
    Second,
    Third,
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

        if self.modal_window_open {
            egui::Window::new("modal")
                //                .open(&mut self.modal_window_open)
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

            egui::Window::new("table").show(ctx, |ui| {
                if let Some((dfname, result)) = &self.hello_promise {
                    if let Some(df) = result.ready() {
                        if let Ok(df) = df {
                            self.dataframes.insert(dfname.clone(), df.clone());
                            display_dataframe(ui, df);
                        } else {
                            default_table(ui)
                        }
                    } else {
                        default_table(ui)
                    }
                } else {
                    default_table(ui);
                };
            });

            egui::Window::new("plot").show(ctx, |ui| {
                let radio = &mut Enum::First;
                ui.horizontal(|ui| {
                    ui.label("Select DataFrame");
                    egui::ComboBox::from_label("")
                        .selected_text(format!("{radio:?}"))
                        .show_ui(ui, |ui| {
                            ui.style_mut().wrap = Some(false);
                            ui.set_min_width(60.0);
                            ui.selectable_value(radio, Enum::First, "First");
                            ui.selectable_value(radio, Enum::Second, "Second");
                            ui.selectable_value(radio, Enum::Third, "Third");
                        });
                });
                ui.separator();
                let plot = egui_plot::Plot::new("lines_demo")
                    .legend(egui_plot::Legend::default())
                    .data_aspect(1.0)
                    //.y_axis_width(4)
                    .x_axis_label("x[m]")
                    .y_axis_label("y[m]")
                    .show_axes(true)
                    .show_grid(true);
                let time: f64 = 1.0;
                let ppoints = if let Some((_, result)) = &self.hello_promise {
                    if let Some(result) = result.ready() {
                        let xs: Vec<f64> = result
                            .as_ref()
                            .unwrap()
                            .column("column_4")
                            .unwrap()
                            .cast(&DataType::Float64)
                            .unwrap()
                            .f64()
                            .unwrap()
                            .into_no_null_iter()
                            .collect();
                        let ys: Vec<f64> = result
                            .as_ref()
                            .unwrap()
                            .column("column_8")
                            .unwrap()
                            .cast(&DataType::Float64)
                            .unwrap()
                            .f64()
                            .unwrap()
                            .into_no_null_iter()
                            .collect();
                        let xys: Vec<[f64; 2]> = (0..xs.len()).map(|i| [xs[i], ys[i]]).collect();
                        egui_plot::PlotPoints::new(xys)
                    } else {
                        egui_plot::PlotPoints::from_explicit_callback(
                            move |x| 0.5 * (2.0 * x).sin() * time.sin(),
                            ..,
                            512,
                        )
                    }
                } else {
                    egui_plot::PlotPoints::from_explicit_callback(
                        move |x| 0.5 * (2.0 * x).sin() * time.sin(),
                        ..,
                        512,
                    )
                };
                plot.show(ui, |plot_ui| {
                    plot_ui.line({
                        egui_plot::Line::new(ppoints)
                            //                        .color(Color32::from_rgb(200, 100, 100))
                            //                       .style(self.line_style)
                            .name("wave")
                    });
                })
            });
            if !self.organized {
                // organize windows
                ui.memory_mut(|mem| mem.reset_areas());
                self.organized = true;
            }
        });
    }
}

use egui_extras::{Column, TableBuilder};

fn display_dataframe(ui: &mut egui::Ui, df: &DataFrame) {
    egui::ScrollArea::both().max_width(500.0).show(ui, |ui| {
        let column_names = df.get_column_names();

        let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
        let table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .columns(Column::auto(), column_names.len() + 1);

        let cols = df.get_columns();
        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("index");
                });
                for cname in df.get_column_names() {
                    header.col(|ui| {
                        ui.strong(cname);
                    });
                }
            })
            .body(|body| {
                body.rows(text_height, cols[0].len(), |row_index, mut row| {
                    row.col(|ui| {
                        ui.strong(row_index.to_string());
                    });
                    for c_idx in 0..column_names.len() {
                        row.col(|ui| {
                            ui.label(cols[c_idx].get(row_index).unwrap().to_string());
                        });
                    }
                });
            });
    });
}

fn default_table(ui: &mut egui::Ui) {
    egui::ScrollArea::both().max_width(500.0).show(ui, |ui| {
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
        let table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::remainder())
            .min_scrolled_height(0.0);

        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("Row");
                });
                header.col(|ui| {
                    ui.strong("Expanding content");
                });
                header.col(|ui| {
                    ui.strong("Clipped text");
                });
                header.col(|ui| {
                    ui.strong("Content");
                });
            })
            .body(|body| {
                body.rows(text_height, 10000, |row_index, mut row| {
                    row.col(|ui| {
                        ui.label(row_index.to_string());
                    });
                    row.col(|ui| {
                        ui.label("test");
                    });
                    row.col(|ui| {
                        ui.label("test");
                    });
                });
            });
    });
}
