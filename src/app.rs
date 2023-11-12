use hello_world::operator_service_client::OperatorServiceClient;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

use serde_derive::{Deserialize, Serialize};
use std::sync::mpsc::{Receiver, Sender};

use poll_promise::Promise;
type SendDataFrameStream = tonic::Streaming<hello_world::DataframeBytes>;
fn send_req_df() -> Promise<Result<DataFrame, tonic::Status>> {
    Promise::spawn_local(async move {
        let base_url = "http://192.168.64.2:50051";
        use tonic_web_wasm_client::Client;
        let mut query_client = hello_world::polars_service_client::PolarsServiceClient::new(
            Client::new(base_url.to_string()),
        );
        let req = hello_world::FilenameRequest {
            filename: "".to_string(),
        };
        let mut stream = query_client.send_dataframe(req).await?.into_inner();

        let mut cvec = Vec::new();
        while let Some(cdata) = stream.message().await? {
            for v in cdata.data {
                cvec.push(v);
            }
        }
        let df = bincode::deserialize_from(cvec.clone().as_slice()).unwrap();
        log::info!("{}", df);
        Ok(df)
    })
}

type OutputCsvStream = tonic::Streaming<hello_world::OperatorCsvFile>;
fn send_req_csv() -> Promise<Result<(), tonic::Status>> {
    log::info!("async!");
    Promise::spawn_local(async move {
        let base_url = "http://192.168.64.2:50051"; // URL of the gRPC-web server
        use tonic_web_wasm_client::Client;
        let mut query_client = OperatorServiceClient::new(Client::new(base_url.to_string()));
        let req = hello_world::OutputCsvOperatorRequest { group_id: 0 };
        let mut stream = query_client.output_csv(req).await?.into_inner();

        let filename = stream.message().await?;
        log::info!("filename: {:?}", filename);

        let mut cvec = Vec::new();
        while let Some(cdata) = stream.message().await? {
            match cdata.value {
                Some(hello_world::operator_csv_file::Value::Data(dv)) => {
                    for v in dv {
                        cvec.push(v);
                    }
                }
                _ => (),
            }
        }
        let rdr = CsvReader::new(std::io::Cursor::new(&cvec));
        let df = rdr.finish().expect("csv reader error");

        log::info!("cvec: {:?}", cvec);
        log::info!("df: {:?}", df);

        Ok(())
    })
}

//use egui_plotter::EguiBackend;
//use plotters::prelude::*;
const MOVE_SCALE: f32 = 0.01;
const SCROLL_SCALE: f32 = 0.001;
use eframe::egui;
use polars::prelude::*;
/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
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

    #[serde(skip)]
    hello_promise: Option<Promise<Result<DataFrame, tonic::Status>>>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
            chart_pitch: 0.3,
            chart_yaw: 0.9,
            chart_scale: 0.9,
            chart_pitch_vel: 0.0,
            chart_yaw_vel: 0.0,
            organized: false,
            hello_promise: None,
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
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(result) = &self.hello_promise {
            if let Some(result) = result.ready() {
                log::info!("got {:?}!!", result);
            }
        }

        // Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
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
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("eframe template");

            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.text_edit_singleline(&mut self.label);
            });

            ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
            if ui.button("Load File").clicked() {
                log::info!("button");
                self.hello_promise = Some(send_req_df());
            }

            ui.separator();

            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/master/",
                "Source code."
            ));

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
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

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
