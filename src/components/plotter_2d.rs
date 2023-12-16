use crate::common_data::{self};
use crate::components::dataframe_select;

use crate::unwrap_or_continue;

use eframe::egui::{self};
use egui_plot::PlotBounds;
use polars::prelude::*;

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Hash, Clone, Copy)]
pub enum SeriesSource {
    DataFrame,
    GRPCClient,
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Hash, Clone, Copy)]
pub enum PlotType {
    Point,
    Pose,
}

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
//#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct SeriesInfo {
    title: String,
    source: SeriesSource,
    plot_type: PlotType,
    df_id: Option<String>,
    visible: bool,
    x_column: Option<String>,
    y_column: Option<String>,
    theta_column: Option<String>,
    marker_radius: f64,
    track_this: bool,
}

impl Default for SeriesInfo {
    fn default() -> Self {
        Self {
            title: "".to_string(),
            source: SeriesSource::DataFrame,
            plot_type: PlotType::Point,
            df_id: None,
            visible: true,
            x_column: None,
            y_column: None,
            theta_column: None,
            marker_radius: 1.0,
            track_this: false,
        }
    }
}

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
//#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Plotter2D {
    title: String,
    equal_aspect: bool,
    series_infos: Vec<SeriesInfo>,
    series_df_selectors: Vec<dataframe_select::DataFrameSelect>,
}

impl Default for Plotter2D {
    fn default() -> Self {
        Self {
            title: "".to_string(),
            equal_aspect: true,
            series_infos: Vec::new(),
            series_df_selectors: Vec::new(),
        }
    }
}

impl Plotter2D {
    fn series_settings(
        idx: usize,
        info: &mut SeriesInfo,
        ui: &mut egui::Ui,
        df: Option<&DataFrame>,
    ) -> Option<()> {
        let df = df?;
        ui.label("x axis: ");
        egui::ComboBox::from_id_source(format!("x_select_{}", idx))
            .selected_text(info.x_column.as_deref().unwrap_or_default())
            .show_ui(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                ui.set_min_width(60.0);
                for (_, &cname) in df.get_column_names().iter().enumerate() {
                    ui.selectable_value(&mut info.x_column, Some(cname.to_string()), cname);
                }
            });
        ui.label("y axis: ");
        egui::ComboBox::from_id_source(format!("y_select_{}", idx))
            .selected_text(info.y_column.as_deref().unwrap_or_default())
            .show_ui(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                ui.set_min_width(60.0);
                for (_, &cname) in df.get_column_names().iter().enumerate() {
                    ui.selectable_value(&mut info.y_column, Some(cname.to_string()), cname);
                }
            });
        if info.plot_type == PlotType::Pose {
            ui.label("yaw: ");
            egui::ComboBox::from_id_source(format!("theta_select_{}", idx))
                .selected_text(info.theta_column.as_deref().unwrap_or_default())
                .show_ui(ui, |ui| {
                    ui.style_mut().wrap = Some(false);
                    ui.set_min_width(60.0);
                    for (_, &cname) in df.get_column_names().iter().enumerate() {
                        ui.selectable_value(&mut info.theta_column, Some(cname.to_string()), cname);
                    }
                });
            ui.add(
                egui::DragValue::new(&mut info.marker_radius)
                    .speed(0.1)
                    .clamp_range(0.0..=f64::INFINITY)
                    .prefix("marker size: "),
            );
        }

        None
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        common_data_arc: std::sync::Arc<std::sync::Mutex<common_data::CommonData>>,
    ) {
        let common_data = common_data_arc.try_lock();
        if common_data.is_err() {
            return;
        }
        let common_data = common_data.unwrap();
        egui::CollapsingHeader::new("Plot Settings")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.title).hint_text("title of the plot"),
                    );
                    ui.checkbox(&mut self.equal_aspect, "Equal Aspect Ratio");
                });

                let mut del_idx = None;
                let mut selector_iter = self.series_df_selectors.iter_mut();
                let num = self.series_infos.len();
                let mut sinfo_iter = self.series_infos.iter_mut();
                for idx in 0..num {
                    let info = sinfo_iter.next().unwrap();
                    ui.separator();

                    let selector = unwrap_or_continue!(selector_iter.next());
                    ui.horizontal(|ui| {
                        ui.push_id(format!("sname_{}", idx), |ui| {
                            ui.label(format!("Series {}, ", idx));
                            ui.add(
                                egui::TextEdit::singleline(&mut info.title)
                                    .hint_text("title of the series"),
                            );
                            if ui.button("delete").clicked() {
                                del_idx = Some(idx);
                            }
                            ui.checkbox(&mut info.visible, "visible");
                            ui.label("source: ");
                        });
                        ui.push_id(format!("df_source_{}", idx), |ui| {
                            egui::ComboBox::from_id_source(idx)
                                .selected_text(match info.source {
                                    SeriesSource::DataFrame => "DataFrme",
                                    SeriesSource::GRPCClient => "GRPC Client",
                                })
                                .show_ui(ui, |ui| {
                                    ui.style_mut().wrap = Some(false);
                                    ui.set_min_width(60.0);
                                    ui.selectable_value(
                                        &mut info.source,
                                        SeriesSource::DataFrame,
                                        "DataFrame",
                                    );
                                    ui.selectable_value(
                                        &mut info.source,
                                        SeriesSource::GRPCClient,
                                        "GRPC Client",
                                    );
                                });
                        });
                    });
                    let mut series_df = None;
                    ui.horizontal(|ui| {
                        let common_data_ref = &(*common_data);
                        series_df = match info.source {
                            SeriesSource::DataFrame => {
                                selector.select_df(idx + 1, ui, common_data_ref)
                            }
                            SeriesSource::GRPCClient => {
                                selector.select_backend_df(idx + 1, ui, common_data_ref)
                            }
                        };
                        ui.label("Plot Type");
                        egui::ComboBox::from_id_source(format!("plot_type_select_{}", idx))
                            .selected_text(match info.plot_type {
                                PlotType::Point => "Point",
                                PlotType::Pose => "Pose",
                            })
                            .show_ui(ui, |ui| {
                                ui.style_mut().wrap = Some(false);
                                ui.set_min_width(60.0);
                                ui.selectable_value(&mut info.plot_type, PlotType::Point, "Point");
                                ui.selectable_value(&mut info.plot_type, PlotType::Pose, "Pose");
                            });
                    });
                    ui.horizontal(|ui| {
                        Plotter2D::series_settings(idx + 1, info, ui, series_df);

                        ui.push_id(format!("track_this_{}", idx), |ui| {
                            ui.checkbox(&mut info.track_this, "track_this");
                        });
                    });
                }

                if let Some(del_idx) = del_idx {
                    self.series_infos.remove(del_idx);
                    self.series_df_selectors.remove(del_idx);
                }

                ui.separator();
                if ui.button("Add Series").clicked() {
                    self.series_infos.push(SeriesInfo::default());
                    self.series_df_selectors
                        .push(dataframe_select::DataFrameSelect::default());
                }
            });

        ui.separator();
        ui.vertical_centered(|ui| {
            ui.label(&self.title);
        });
        let plot = egui_plot::Plot::new("lines_demo")
            .legend(egui_plot::Legend::default())
            //.y_axis_width(4)
            .x_axis_label("x[m]")
            .y_axis_label("y[m]")
            .auto_bounds_x()
            .auto_bounds_y()
            .show_axes(true)
            .show_grid(true);
        let plot = if self.equal_aspect {
            plot.data_aspect(1.0)
        } else {
            plot
        };

        let extract_series = |df: &DataFrame, cname: &str| -> Vec<f64> {
            let col = df.column(cname);
            if let Ok(col) = col {
                col.cast(&DataType::Float64)
                    .unwrap()
                    .f64()
                    .unwrap()
                    .into_no_null_iter()
                    .collect()
            } else {
                Vec::new()
            }
        };

        plot.show(ui, |plot_ui| {
            let mut df_select_iter = self.series_df_selectors.iter();
            for s_info in self.series_infos.iter() {
                let df_id = unwrap_or_continue!(unwrap_or_continue!(df_select_iter.next())
                    .dataframe_key
                    .clone());
                if !s_info.visible {
                    continue;
                }
                let local_df = match s_info.source {
                    SeriesSource::DataFrame => common_data
                        .dataframes
                        .get(&df_id)
                        .map(|df_opt_pair| df_opt_pair.1.as_ref().map(|df_opt| df_opt.clone()))
                        .unwrap_or_default(),
                    SeriesSource::GRPCClient => common_data
                        .realtime_dataframes
                        .get(&df_id)
                        .map(|df_opt| df_opt.clone()),
                };
                let local_df = unwrap_or_continue!(local_df);
                let colx = unwrap_or_continue!(&s_info.x_column);
                let coly = unwrap_or_continue!(&s_info.y_column);
                let xs = extract_series(&local_df, colx.as_str());
                let ys = extract_series(&local_df, coly.as_str());
                let xys: Vec<[f64; 2]> = (0..xs.len()).map(|i| [xs[i], ys[i]]).collect();
                //plot_ui.points(points)
                if s_info.track_this {
                    if let (Some(lx), Some(ly)) = (xs.last(), ys.last()) {
                        let range = 0.1;
                        plot_ui.set_plot_bounds(PlotBounds::from_min_max(
                            [lx - range, ly - range],
                            [lx + range, ly + range],
                        ));
                    }
                }
                match s_info.plot_type {
                    PlotType::Point => {
                        plot_ui.points(
                            egui_plot::Points::new(xys)
                                .radius(10.0)
                                .filled(false)
                                .name(&s_info.title),
                        );
                    }
                    PlotType::Pose => {
                        let col_theta = unwrap_or_continue!(&s_info.theta_column);
                        let ts = extract_series(&local_df, col_theta.as_str());
                        let xys2: Vec<[f64; 2]> = (0..xs.len())
                            .map(|i| {
                                [
                                    xs[i] + s_info.marker_radius * ts[i].cos(),
                                    ys[i] + s_info.marker_radius * ts[i].sin(),
                                ]
                            })
                            .collect();
                        let arrows = egui_plot::Arrows::new(xys.clone(), xys2);
                        plot_ui.arrows(arrows);

                        plot_ui.points(
                            egui_plot::Points::new(xys)
                                .radius(10.0)
                                .filled(false)
                                .name(&s_info.title),
                        );
                    }
                }
            }
        });
    }
}
