use crate::common_data;
use eframe::egui;
use polars::prelude::*;

use super::modal_window::get_filename;

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DataFrameSelect {
    pub dataframe_key: Option<String>,
}

impl Default for DataFrameSelect {
    fn default() -> Self {
        Self {
            dataframe_key: None,
        }
    }
}

impl DataFrameSelect {
    pub fn select_df<'a>(
        &mut self,
        idx: usize,
        ui: &mut egui::Ui,
        common_data: &'a mut common_data::CommonData,
    ) -> Option<&'a mut DataFrame> {
        let df_key = self.dataframe_key.clone().unwrap_or("0".to_string());
        ui.push_id(format!("df_select_{}", idx), |ui| {
            ui.horizontal(|ui| -> Option<()> {
                if common_data.dataframes.len() == 0 {
                    ui.label("Load DataFrame");
                } else {
                    let df_opt = common_data.dataframes.get(&df_key);
                    let (df_info, _) = if let Some(df_pair) = df_opt {
                        df_pair
                    } else {
                        let df_key = "0".to_string();
                        common_data.dataframes.get(&df_key).unwrap()
                    };
                    let fname = get_filename(df_info.filepath.as_str());
                    ui.label("Select DataFrame");
                    egui::ComboBox::from_label("")
                        .selected_text(format!("{}", fname))
                        .show_ui(ui, |ui| {
                            ui.style_mut().wrap = Some(false);
                            ui.set_min_width(60.0);
                            for (i, (df_info, _)) in common_data.dataframes.iter() {
                                let fname = get_filename(df_info.filepath.as_str());
                                ui.selectable_value(
                                    &mut self.dataframe_key,
                                    Some(i.to_string()),
                                    fname,
                                );
                            }
                        });
                }
                None
            });
        });
        Some(common_data.dataframes.get_mut(&df_key)?.1.as_mut()?)
    }
}
