use core::panic;

use crate::{common_data, components::dataframe_select};
use eframe::egui;
use egui_extras::{Column, TableBuilder};
use polars::prelude::*;

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
struct ColumnSelectUi {
    column: Option<String>,
}

impl ColumnSelectUi {
    fn new() -> Self {
        Self { column: None }
    }
    fn show(&mut self, df: &mut DataFrame, ui: &mut egui::Ui, id_source: String) {
        egui::ComboBox::from_id_source(id_source)
            .selected_text(self.column.clone().unwrap_or_default())
            .show_ui(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                ui.set_min_width(60.0);
                for (_, &cname) in df.get_column_names().iter().enumerate() {
                    let mut column = self.column.take().unwrap_or_default();
                    ui.selectable_value(&mut column, cname.to_string(), cname);
                    self.column = Some(column);
                }
            });
    }
}

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
struct ENUTransform {
    ui_lat: ColumnSelectUi,
    ui_lon: ColumnSelectUi,
}

impl ENUTransform {
    fn new() -> Self {
        Self {
            ui_lat: ColumnSelectUi::new(),
            ui_lon: ColumnSelectUi::new(),
        }
    }
    fn show(&mut self, df: &mut DataFrame, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("ENU Transform: latitude: ");
            self.ui_lat.show(df, ui, format!("ENU_trfm_lat"));
            ui.label(", longitude: ");
            self.ui_lon.show(df, ui, format!("ENU_trfm_lon"));
            if ui.button("execute").clicked() {
                let df_clone = df.clone();
                let lat_col = df_clone
                    .column(self.ui_lat.column.as_ref().unwrap().as_str())
                    .unwrap()
                    .f64()
                    .unwrap();
                let lon_col = df_clone
                    .column(self.ui_lon.column.as_ref().unwrap().as_str())
                    .unwrap()
                    .f64()
                    .unwrap();
                let hei_col = df_clone
                    .column(" Ellipsoid Height (m)")
                    .unwrap()
                    .f64()
                    .unwrap();

                let res: Vec<(f64, f64, f64)> = lat_col
                    .into_iter()
                    .zip(lon_col.into_iter())
                    .zip(hei_col.into_iter())
                    .map(|(latlon, hei)| match (latlon, hei) {
                        ((Some(lat), Some(lon)), Some(hei)) => {
                            let (pe, pn, pu) = map_3d::geodetic2enu(
                                lat.to_radians(),
                                lon.to_radians(),
                                hei,
                                lat_col.get(0).unwrap().to_radians(),
                                lon_col.get(0).unwrap().to_radians(),
                                hei_col.get(0).unwrap(),
                                map_3d::Ellipsoid::WGS84,
                            );
                            (pe, pn, pu)
                        }
                        _ => panic!("unexpected"),
                    })
                    .collect();
                let mut e_vec = Vec::new();
                let mut n_vec = Vec::new();
                let mut u_vec = Vec::new();
                for (e, n, u) in res.iter() {
                    e_vec.push(*e);
                    n_vec.push(*n);
                    u_vec.push(*u);
                }

                df.with_column(Series::new("ENU_E[m]", e_vec)).unwrap();
                df.with_column(Series::new("ENU_N[m]", n_vec)).unwrap();
                df.with_column(Series::new("ENU_U[m]", u_vec)).unwrap();
            }
        });
    }
}

pub fn show_dataframe_table(ui: &mut egui::Ui, df: &DataFrame) {
    egui::ScrollArea::both().show(ui, |ui| {
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
                                    ui.label(cols[c_idx].get(row_index).unwrap().to_string());
                                });
                            }
                        }
                    },
                );
            });
    });
}

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DataFrameTablePane {
    dataframe_select: dataframe_select::DataFrameSelect,
    df_manip_enu_transform: ENUTransform,
}

impl Default for DataFrameTablePane {
    fn default() -> Self {
        Self {
            dataframe_select: dataframe_select::DataFrameSelect::default(),
            df_manip_enu_transform: ENUTransform::new(),
        }
    }
}

impl DataFrameTablePane {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        common_data_arc: std::sync::Arc<std::sync::Mutex<common_data::CommonData>>,
    ) -> Option<()> {
        let common_data = &mut common_data_arc.lock().ok()?;

        let mut df = self.dataframe_select.select_df(0, ui, common_data)?;

        egui::CollapsingHeader::new("Column-wise Operations")
            .default_open(true)
            .show(ui, |ui| {
                self.df_manip_enu_transform.show(&mut df, ui);
            });

        show_dataframe_table(ui, df);
        None
    }
}
