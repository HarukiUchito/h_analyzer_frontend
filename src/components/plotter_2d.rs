use crate::backend_talk::grpc_data_transfer;
use crate::common_data::{self};
use crate::components::dataframe_select;
use eframe::egui;
use polars::prelude::*;
use std::collections::LinkedList;

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Hash, Clone, Copy)]
pub enum SeriesSource {
    DATAFRAME,
    REALTIME_COMMAND,
}

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
//#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct SeriesInfo {
    source: SeriesSource,
    df_id: Option<String>,
    x_column: Option<String>,
    y_column: Option<String>,
}

impl Default for SeriesInfo {
    fn default() -> Self {
        Self {
            source: SeriesSource::DATAFRAME,
            df_id: None,
            x_column: None,
            y_column: None,
        }
    }
}

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
//#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Plotter2D {
    common_dataframe_select: dataframe_select::DataFrameSelect,

    equal_aspect: bool,
    series_infos: LinkedList<SeriesInfo>,
    series_df_selectors: LinkedList<dataframe_select::DataFrameSelect>,
}

impl Default for Plotter2D {
    fn default() -> Self {
        Self {
            common_dataframe_select: dataframe_select::DataFrameSelect::default(),
            equal_aspect: true,
            series_infos: LinkedList::new(),
            series_df_selectors: LinkedList::new(),
        }
    }
}

impl Plotter2D {
    fn series_settings(
        idx: usize,
        info: &mut SeriesInfo,
        ui: &mut egui::Ui,
        df: Option<&DataFrame>,
        common_data: &common_data::CommonData,
    ) {
        if let Some(df) = df {
            ui.label("x axis: ");
            ui.push_id(idx, |ui| {
                egui::ComboBox::from_id_source(format!("x_select_{}", idx))
                    .selected_text(info.x_column.as_deref().unwrap_or_default())
                    .show_ui(ui, |ui| {
                        ui.style_mut().wrap = Some(false);
                        ui.set_min_width(60.0);
                        for (i, &cname) in df.get_column_names().iter().enumerate() {
                            ui.selectable_value(&mut info.x_column, Some(cname.to_string()), cname);
                        }
                    });
                ui.label("y axis: ");
                egui::ComboBox::from_id_source(format!("y_select_{}", idx))
                    .selected_text(info.y_column.as_deref().unwrap_or_default())
                    .show_ui(ui, |ui| {
                        ui.style_mut().wrap = Some(false);
                        ui.set_min_width(60.0);
                        for (i, &cname) in df.get_column_names().iter().enumerate() {
                            ui.selectable_value(&mut info.y_column, Some(cname.to_string()), cname);
                        }
                    });
            });
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, common_data: &common_data::CommonData) {
        egui::Window::new("plot").show(ctx, |ui| {
            let df = self.common_dataframe_select.select_df(0, ui, common_data);
            egui::CollapsingHeader::new("Plot Settings")
                .default_open(true)
                .show(ui, |ui| {
                    ui.checkbox(&mut self.equal_aspect, "Equal Aspect Ratio");

                    let mut selector_iter = self.series_df_selectors.iter_mut();
                    for (idx, info) in self.series_infos.iter_mut().enumerate() {
                        ui.separator();

                        if let Some(selector) = selector_iter.next() {
                            ui.horizontal(|ui| {
                                ui.push_id(idx, |ui| {
                                    ui.label(format!("Series {}, ", idx));
                                    ui.label("source: ");
                                });
                                ui.push_id(format!("df_source_{}", idx), |ui| {
                                    egui::ComboBox::from_id_source(idx)
                                        .selected_text(match info.source {
                                            SeriesSource::DATAFRAME => "DataFrme",
                                            SeriesSource::REALTIME_COMMAND => "Real Time Command",
                                        })
                                        .show_ui(ui, |ui| {
                                            ui.style_mut().wrap = Some(false);
                                            ui.set_min_width(60.0);
                                            ui.selectable_value(
                                                &mut info.source,
                                                SeriesSource::DATAFRAME,
                                                "DataFrame",
                                            );
                                            ui.selectable_value(
                                                &mut info.source,
                                                SeriesSource::REALTIME_COMMAND,
                                                "Realtime Command",
                                            );
                                        });
                                });
                                let series_df = match info.source {
                                    SeriesSource::DATAFRAME => {
                                        selector.select_df(idx + 1, ui, common_data)
                                    }
                                    SeriesSource::REALTIME_COMMAND => {
                                        selector.select_backend_df(idx + 1, ui, common_data)
                                    }
                                };

                                Plotter2D::series_settings(
                                    idx + 1,
                                    info,
                                    ui,
                                    series_df,
                                    common_data,
                                );
                            });
                        }
                    }

                    ui.separator();
                    if ui.button("Add Series").clicked() {
                        self.series_infos.push_back(SeriesInfo::default());
                        self.series_df_selectors
                            .push_back(dataframe_select::DataFrameSelect::default());
                    }
                });

            ui.separator();
            let plot = egui_plot::Plot::new("lines_demo")
                .legend(egui_plot::Legend::default())
                //.y_axis_width(4)
                .x_axis_label("x[m]")
                .y_axis_label("y[m]")
                .show_axes(true)
                .show_grid(true);
            let plot = if self.equal_aspect {
                plot.data_aspect(1.0)
            } else {
                plot
            };

            let extract_series = (|df: &DataFrame, cname: &str| -> Vec<f64> {
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
            });

            if let Some(df) = df {
                let time: f64 = 1.0;
                let ppoints = if false && !df.is_empty() {
                    let xs = extract_series(&df, "column_4");
                    let ys = extract_series(&df, "column_8");
                    let xys: Vec<[f64; 2]> = (0..xs.len()).map(|i| [xs[i], ys[i]]).collect();
                    egui_plot::PlotPoints::new(xys)
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
                    for s_info in self.series_infos.iter() {
                        if let (Some(colx), Some(coly)) = (&s_info.x_column, &s_info.y_column) {
                            let xs = extract_series(&df, colx.as_str());
                            let ys = extract_series(&df, coly.as_str());
                            let xys: Vec<[f64; 2]> =
                                (0..xs.len()).map(|i| [xs[i], ys[i]]).collect();
                            let ppoints = egui_plot::PlotPoints::new(xys);
                            plot_ui.line(egui_plot::Line::new(ppoints).name(coly.as_str()));
                        }
                    }
                });
            }
        });
    }
}
