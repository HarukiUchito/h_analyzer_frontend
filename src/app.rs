#[derive(serde::Deserialize, serde::Serialize, PartialEq)]
enum DataFrameType {
    NDEV,
    KITTI,
}

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
    // Example stuff:
    label: String,

    #[serde(skip)] // This how you opt-out of serialization of a field
    value: f32,

    chart_pitch: f32,
    chart_yaw: f32,
    chart_scale: f32,
    chart_pitch_vel: f32,
    chart_yaw_vel: f32,
    organized: bool,

    modal_window_open: bool,
    dataframe_type: Option<DataFrameType>,
    filepath_to_be_loaded: Option<String>,

    #[serde(skip)]
    hello_promise: Option<Promise<Result<DataFrame, tonic::Status>>>,

    current_path: String,
    default_path: String,
    #[serde(skip)]
    fs_list_promise: Option<Promise<Result<grpc_fs::ListResponse, tonic::Status>>>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let path = "/home/haruki/data/dataset/poses/";
        let backend = backend_talk::BackendTalk::default();
        let fs_list_promise = backend.request_list(path.to_string());
        Self {
            backend: backend,
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
            chart_pitch: 0.3,
            chart_yaw: 0.9,
            chart_scale: 0.9,
            chart_pitch_vel: 0.0,
            chart_yaw_vel: 0.0,
            organized: false,
            modal_window_open: false,
            dataframe_type: None,
            filepath_to_be_loaded: None,
            hello_promise: None,
            current_path: path.to_owned(),
            default_path: path.to_owned(),
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
        if self.modal_window_open {
            egui::Window::new("modal")
                //                .open(&mut self.modal_window_open)
                .show(ctx, |ui| {
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
                                Some(self.backend.load_df_request(filepath.clone()));
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
            ui.heading("eframe template");

            ui.separator();

            if ui.button("refresh").clicked() {
                self.fs_list_promise = Some(self.backend.request_list(self.current_path.clone()));
            }
            ui.separator();

            ui.label(self.current_path.as_str());

            ui.separator();

            egui::ScrollArea::both().show(ui, |ui| {
                let mut update_list = false;
                if let Some(fs_list) = &self.fs_list_promise {
                    if let Some(fs_list) = fs_list.ready() {
                        if let Ok(fs_list) = fs_list {
                            let mut b1 = false;
                            let dirname = "..";
                            if ui.checkbox(&mut b1, dirname).double_clicked() {
                                let nfp =
                                    std::path::Path::new(self.current_path.as_str()).join(dirname);
                                self.current_path = nfp.to_string_lossy().into_owned();
                                update_list = true;
                            }
                            for dirname in fs_list.directories.iter() {
                                if ui.checkbox(&mut b1, dirname).double_clicked() {
                                    let nfp = std::path::Path::new(self.current_path.as_str())
                                        .join(dirname);
                                    self.current_path = nfp.to_string_lossy().into_owned();
                                    update_list = true;
                                }
                            }
                            for filename in fs_list.files.iter() {
                                let mut b1 = false;
                                if ui.checkbox(&mut b1, filename).double_clicked() {
                                    self.modal_window_open = true;
                                    let nfp = std::path::Path::new(self.current_path.as_str())
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
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_enabled(!self.modal_window_open);

            egui::Window::new("test3").show(ctx, |ui| {});
            /*
                        egui::Window::new("test").show(ctx, |ui| {
                            // First, get mouse data
                            let (pitch_delta, yaw_delta, scale_delta) = ui.input(|input| {
                                let pointer = &input.pointer;
                                let delta = pointer.delta();

                                let (pitch_delta, yaw_delta) = match pointer.primary_down() {
                                    true => (delta.y * MOVE_SCALE, -delta.x * MOVE_SCALE),
                                    false => (self.chart_pitch_vel, self.chart_yaw_vel),
                                };

                                let scale_delta = input.scroll_delta.y * SCROLL_SCALE;

                                (pitch_delta, yaw_delta, scale_delta)
                            });

                            self.chart_pitch_vel = pitch_delta;
                            self.chart_yaw_vel = yaw_delta;

                            self.chart_pitch += self.chart_pitch_vel;
                            self.chart_yaw += self.chart_yaw_vel;
                            self.chart_scale += scale_delta;

                            // Next plot everything
                            let root = EguiBackend::new(ui).into_drawing_area();

                            root.fill(&WHITE).unwrap();

                            let x_axis = (-3.0..3.0).step(0.1);
                            let z_axis = (-3.0..3.0).step(0.1);

                            let mut chart = ChartBuilder::on(&root)
                                .caption(format!("3D Plot Test"), (FontFamily::SansSerif, 20))
                                .build_cartesian_3d(x_axis, -3.0..3.0, z_axis)
                                .unwrap();

                            chart.with_projection(|mut pb| {
                                pb.yaw = self.chart_yaw as f64;
                                pb.pitch = self.chart_pitch as f64;
                                pb.scale = self.chart_scale as f64;
                                pb.into_matrix()
                            });

                            chart
                                .configure_axes()
                                .light_grid_style(BLACK.mix(0.15))
                                .max_light_lines(3)
                                .draw()
                                .unwrap();

                            chart
                                .draw_series(
                                    SurfaceSeries::xoz(
                                        (-30..30).map(|f| f as f64 / 10.0),
                                        (-30..30).map(|f| f as f64 / 10.0),
                                        |x, z| (x * x + z * z).cos(),
                                    )
                                    .style(BLUE.mix(0.2).filled()),
                                )
                                .unwrap()
                                .label("Surface")
                                .legend(|(x, y)| {
                                    Rectangle::new([(x + 5, y - 5), (x + 15, y + 5)], BLUE.mix(0.5).filled())
                                });

                            chart
                                .draw_series(LineSeries::new(
                                    (-100..100)
                                        .map(|y| y as f64 / 40.0)
                                        .map(|y| ((y * 10.0).sin(), y, (y * 10.0).cos())),
                                    &BLACK,
                                ))
                                .unwrap()
                                .label("Line")
                                .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLACK));

                            chart
                                .configure_series_labels()
                                .border_style(BLACK)
                                .draw()
                                .unwrap();

                            root.present().unwrap();
                        });
            */
            egui::Window::new("test2").show(ctx, |ui| {
                let mut plot = egui_plot::Plot::new("lines_demo")
                    .legend(egui_plot::Legend::default())
                    .y_axis_width(4)
                    .show_axes(true)
                    .show_grid(true);
                let time: f64 = 1.0;
                let ppoints = if let Some(result) = &self.hello_promise {
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
