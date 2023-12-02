use crate::common_data::{self};
use crate::components::dataframe_select;
use eframe::egui;
use polars::prelude::*;

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Hash, Clone, Copy)]
pub enum SeriesSource {
    DATAFRAME,
    REALTIME_COMMAND,
}

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
//#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct SeriesInfo {
    source: SeriesSource,
    x_column: Option<String>,
    y_column: Option<String>,
}

impl Default for SeriesInfo {
    fn default() -> Self {
        Self {
            source: SeriesSource::DATAFRAME,
            x_column: None,
            y_column: None,
        }
    }
}

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
//#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Plotter2D {
    dataframe_select: dataframe_select::DataFrameSelect,

    equal_aspect: bool,
    series_infos: Vec<SeriesInfo>,
}

impl Default for Plotter2D {
    fn default() -> Self {
        Self {
            dataframe_select: dataframe_select::DataFrameSelect::default(),
            equal_aspect: true,
            series_infos: Vec::new(),
        }
    }
}

impl Plotter2D {
    pub fn show(&mut self, ctx: &egui::Context, common_data: &common_data::CommonData) {
        egui::Window::new("plot").show(ctx, |ui| {
            let df = self.dataframe_select.select(ui, common_data);
            egui::CollapsingHeader::new("Plot Settings")
                .default_open(true)
                .show(ui, |ui| {
                    ui.checkbox(&mut self.equal_aspect, "Equal Aspect Ratio");

                    for (idx, info) in self.series_infos.iter_mut().enumerate() {
                        ui.push_id(idx, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(format!("Series {}, ", idx));
                                ui.label("source: ");
                                egui::ComboBox::from_id_source(0)
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
                                ui.label("x axis: ");
                                egui::ComboBox::from_id_source(1)
                                    .selected_text(info.x_column.as_deref().unwrap_or_default())
                                    .show_ui(ui, |ui| {
                                        ui.style_mut().wrap = Some(false);
                                        ui.set_min_width(60.0);
                                        for (i, &cname) in df.get_column_names().iter().enumerate()
                                        {
                                            ui.selectable_value(
                                                &mut info.x_column,
                                                Some(cname.to_string()),
                                                cname,
                                            );
                                        }
                                    });
                                ui.label("y axis: ");
                                egui::ComboBox::from_id_source(2)
                                    .selected_text(info.y_column.as_deref().unwrap_or_default())
                                    .show_ui(ui, |ui| {
                                        ui.style_mut().wrap = Some(false);
                                        ui.set_min_width(60.0);
                                        for (i, &cname) in df.get_column_names().iter().enumerate()
                                        {
                                            ui.selectable_value(
                                                &mut info.y_column,
                                                Some(cname.to_string()),
                                                cname,
                                            );
                                        }
                                    });
                            });
                        });
                    }

                    if ui.button("Add Series").clicked() {
                        self.series_infos.push(SeriesInfo::default());
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
                        let xys: Vec<[f64; 2]> = (0..xs.len()).map(|i| [xs[i], ys[i]]).collect();
                        let ppoints = egui_plot::PlotPoints::new(xys);
                        plot_ui.line({ egui_plot::Line::new(ppoints).name(coly.as_str()) });
                    }
                }
            })
        });
    }
}
