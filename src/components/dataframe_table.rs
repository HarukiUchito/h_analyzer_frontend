use crate::{common_data, components::modal_window::get_filename};
use eframe::egui;
use egui_extras::{Column, TableBuilder};
use polars::prelude::*;

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct DataFrameTable {
    dataframe_index: usize,
}

impl Default for DataFrameTable {
    fn default() -> Self {
        Self { dataframe_index: 0 }
    }
}

impl DataFrameTable {
    pub fn show(&mut self, ctx: &egui::Context, common_data: &common_data::CommonData) {
        egui::Window::new("table").show(ctx, |ui| {
            let df = (|| {
                ui.horizontal(|ui| {
                    if common_data.dataframes.len() == 0 {
                        ui.label("Load DataFrame");
                    } else {
                        let (df_info, _) =
                            common_data.dataframes.get(self.dataframe_index).unwrap();
                        let fname = get_filename(df_info.filepath.as_str());
                        ui.label("Select DataFrame");
                        egui::ComboBox::from_label("")
                            .selected_text(format!("{}", fname))
                            .show_ui(ui, |ui| {
                                ui.style_mut().wrap = Some(false);
                                ui.set_min_width(60.0);
                                for (i, (df_info, df)) in common_data.dataframes.iter().enumerate()
                                {
                                    if let Some(_) = df {
                                        let fname = get_filename(df_info.filepath.as_str());
                                        ui.selectable_value(&mut self.dataframe_index, i, fname);
                                    }
                                }
                            });
                    }
                });
                if let Some(tp) = common_data.dataframes.get(self.dataframe_index) {
                    if let Some(df) = &tp.1 {
                        return df.clone();
                    }
                }
                DataFrame::default()
            })();
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
                        body.rows(
                            text_height,
                            if df.is_empty() { 0 } else { cols[0].len() },
                            |row_index, mut row| {
                                row.col(|ui| {
                                    ui.strong(row_index.to_string());
                                });
                                if !df.is_empty() {
                                    for c_idx in 0..column_names.len() {
                                        row.col(|ui| {
                                            ui.label(
                                                cols[c_idx].get(row_index).unwrap().to_string(),
                                            );
                                        });
                                    }
                                }
                            },
                        );
                    });
            });
        });
    }
}
