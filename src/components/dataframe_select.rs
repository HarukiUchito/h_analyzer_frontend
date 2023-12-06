use crate::common_data;
use eframe::egui;
use polars::prelude::*;

use super::modal_window::get_filename;

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DataFrameSelect {
    dataframe_key: Option<String>,
}

impl Default for DataFrameSelect {
    fn default() -> Self {
        Self {
            dataframe_key: None,
        }
    }
}

impl DataFrameSelect {
    pub fn select_backend_df<'a>(
        &mut self,
        idx: usize,
        ui: &mut egui::Ui,
        common_data: &'a common_data::CommonData,
    ) -> Option<&'a DataFrame> {
        if common_data.realtime_dataframes.len() == 0 {
            ui.label("Load DataFrame");
            None
        } else {
            let df_key = if let Some(df_key) = &self.dataframe_key {
                df_key.clone()
            } else {
                let f_key = common_data
                    .realtime_dataframes
                    .keys()
                    .into_iter()
                    .next()
                    .unwrap()
                    .clone();

                self.dataframe_key = Some(f_key.clone());
                f_key
            };

            let ret_df = common_data.realtime_dataframes.get(&df_key);

            ui.push_id(format!("df_backend_select_{}", idx), |ui| {
                ui.horizontal(|ui| {
                    if let Some(df) = common_data.realtime_dataframes.get(&df_key) {
                        ui.label("Select DataFrame");
                        egui::ComboBox::from_label("")
                            .selected_text(format!("{}", df_key))
                            .show_ui(ui, |ui| {
                                ui.style_mut().wrap = Some(false);
                                ui.set_min_width(60.0);
                                for (id, _) in common_data.realtime_dataframes.iter() {
                                    ui.selectable_value(
                                        &mut self.dataframe_key,
                                        Some(id.to_string()),
                                        id,
                                    );
                                }
                            });
                    }
                });
            });

            ret_df
        }
    }

    pub fn select_df<'a>(
        &mut self,
        idx: usize,
        ui: &mut egui::Ui,
        common_data: &'a common_data::CommonData,
    ) -> Option<&'a DataFrame> {
        let df_key = self.dataframe_key.clone().unwrap_or("0".to_string());
        ui.push_id(format!("df_select_{}", idx), |ui| {
            ui.horizontal(|ui| {
                if common_data.dataframes.len() == 0 {
                    ui.label("Load DataFrame");
                } else {
                    if let Some(df_pair) = common_data.dataframes.get(&df_key) {
                        let (df_info, _) = df_pair;
                        let fname = get_filename(df_info.filepath.as_str());
                        ui.label("Select DataFrame");
                        egui::ComboBox::from_label("")
                            .selected_text(format!("{}", fname))
                            .show_ui(ui, |ui| {
                                ui.style_mut().wrap = Some(false);
                                ui.set_min_width(60.0);
                                for (i, (df_info, df)) in common_data.dataframes.iter() {
                                    if let Some(_) = df {
                                        let fname = get_filename(df_info.filepath.as_str());
                                        ui.selectable_value(
                                            &mut self.dataframe_key,
                                            Some(i.to_string()),
                                            fname,
                                        );
                                    }
                                }
                            });
                    }
                }
            });
        });
        if let Some(tp) = common_data.dataframes.get(&df_key) {
            if let Some(df) = &tp.1 {
                return Some(df);
            }
        }
        None
    }
}
