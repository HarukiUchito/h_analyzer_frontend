use crate::common_data::{self, CommonData};
use crate::components::dataframe_select;
use eframe::egui;
use polars::prelude::*;

use super::modal_window;

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Plotter2D {
    dataframe_select: dataframe_select::DataFrameSelect,
}

impl Default for Plotter2D {
    fn default() -> Self {
        Self {
            dataframe_select: dataframe_select::DataFrameSelect::default(),
        }
    }
}

impl Plotter2D {
    pub fn show(&mut self, ctx: &egui::Context, common_data: &common_data::CommonData) {
        egui::Window::new("plot").show(ctx, |ui| {
            let df = self.dataframe_select.select(ui, common_data);
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
            let ppoints = if !df.is_empty() {
                let xs: Vec<f64> = df
                    .column("column_4")
                    .unwrap()
                    .cast(&DataType::Float64)
                    .unwrap()
                    .f64()
                    .unwrap()
                    .into_no_null_iter()
                    .collect();
                let ys: Vec<f64> = df
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
    }
}
