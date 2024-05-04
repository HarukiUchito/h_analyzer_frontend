use std::borrow::BorrowMut;

use crate::common_data;
use eframe::egui;
use polars::prelude::*;

use super::modal_window::get_filename;

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DataFrameSelect {
    pub dataframe_id: Option<usize>,
}

impl Default for DataFrameSelect {
    fn default() -> Self {
        Self { dataframe_id: None }
    }
}

impl DataFrameSelect {
    pub fn select_df<'a>(
        &mut self,
        idx: usize,
        ui: &mut egui::Ui,
        common_data: &'a mut common_data::CommonData,
    ) -> Option<&'a mut DataFrame> {
        let df_id = self.dataframe_id.clone().unwrap_or(1);
        ui.push_id(format!("df_select_{}", idx), |ui| {
            ui.horizontal(|ui| -> Option<()> {
                if common_data.latest_df_info_map.len() == 0 {
                    ui.label("Load DataFrame");
                } else {
                    let df_info = common_data.latest_df_info_map.get(&df_id).clone().unwrap();
                    let fname = get_filename(df_info.df_path.as_str());
                    ui.label("Select DataFrame");
                    egui::ComboBox::from_label("")
                        .selected_text(format!("{}", fname))
                        .show_ui(ui, |ui| {
                            ui.style_mut().wrap = Some(false);
                            ui.set_min_width(60.0);
                            for (id, df_info) in common_data.latest_df_info_map.iter() {
                                let fname = get_filename(df_info.df_path.as_str());
                                ui.selectable_value(&mut self.dataframe_id, Some(*id), fname);
                            }
                        });
                }
                None
            });
        });

        // request if the df is not available
        common_data.request_df_transmission(df_id);

        Some(common_data.required_dataframes.get_mut(&df_id)?.as_mut()?)
    }
}
