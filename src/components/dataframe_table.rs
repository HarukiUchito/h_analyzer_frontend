use crate::{common_data, components::dataframe_select};
use eframe::egui;
use egui_extras::{Column, TableBuilder};

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct DataFrameTable {
    dataframe_select: dataframe_select::DataFrameSelect,
}

impl Default for DataFrameTable {
    fn default() -> Self {
        Self {
            dataframe_select: dataframe_select::DataFrameSelect::default(),
        }
    }
}

impl DataFrameTable {
    pub fn show(&mut self, ctx: &egui::Context, common_data: &common_data::CommonData) {
        egui::Window::new("table").show(ctx, |ui| {
            let df = self.dataframe_select.select_df(0, ui, common_data);
            if let Some(df) = df {
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
            }
        });
    }
}
