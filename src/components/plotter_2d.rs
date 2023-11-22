use eframe::egui;
use polars::prelude::*;

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Plotter2D {}

impl Default for Plotter2D {
    fn default() -> Self {
        Self {}
    }
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum Enum {
    First,
    Second,
    Third,
}

impl Plotter2D {
    pub fn show(&mut self, ctx: &egui::Context, df: &DataFrame) {
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
