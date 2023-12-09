use crate::common_data::{self};
use crate::components::dataframe_select;

use crate::unwrap_or_continue;

use eframe::egui;
use egui_plot::PlotBounds;
use polars::prelude::*;

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Hash, Clone, Copy)]
pub enum SeriesSource {
    DataFrame,
    GRPCClient,
}

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
//#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct SeriesInfo {
    source: SeriesSource,
    df_id: Option<String>,
    visible: bool,
    x_column: Option<String>,
    y_column: Option<String>,
    track_this: bool,
}

impl Default for SeriesInfo {
    fn default() -> Self {
        Self {
            source: SeriesSource::DataFrame,
            df_id: None,
            visible: true,
            x_column: None,
            y_column: None,
            track_this: false,
        }
    }
}

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
//#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Plotter2D {
    equal_aspect: bool,
    series_infos: Vec<SeriesInfo>,
    series_df_selectors: Vec<dataframe_select::DataFrameSelect>,
}

impl Default for Plotter2D {
    fn default() -> Self {
        Self {
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
        ui.push_id(idx, |ui| {
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
        });
        None
    }

    pub fn show(&mut self, ctx: &egui::Context, common_data: &common_data::CommonData) {
        egui::Window::new("plot").show(ctx, |ui| {
            egui::CollapsingHeader::new("Plot Settings")
                .default_open(true)
                .show(ui, |ui| {
                    ui.checkbox(&mut self.equal_aspect, "Equal Aspect Ratio");

                    let mut del_idx = None;
                    let mut selector_iter = self.series_df_selectors.iter_mut();
                    let num = self.series_infos.len();
                    let mut sinfo_iter = self.series_infos.iter_mut();
                    for idx in 0..num {
                        let info = sinfo_iter.next().unwrap();
                        ui.separator();

                        let selector = unwrap_or_continue!(selector_iter.next());
                        ui.horizontal(|ui| {
                            ui.push_id(idx, |ui| {
                                ui.label(format!("Series {}, ", idx));
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
                        ui.horizontal(|ui| {
                            let series_df = match info.source {
                                SeriesSource::DataFrame => {
                                    selector.select_df(idx + 1, ui, common_data)
                                }
                                SeriesSource::GRPCClient => {
                                    selector.select_backend_df(idx + 1, ui, common_data)
                                }
                            };
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
                    plot_ui.points(
                        egui_plot::Points::new(xys)
                            .radius(10.0)
                            .filled(false)
                            .name(coly.as_str()),
                    );
                }
            });
        });
    }
}
