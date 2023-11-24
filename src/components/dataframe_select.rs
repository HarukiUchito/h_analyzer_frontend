use crate::common_data;
use eframe::egui;
use polars::prelude::*;

use super::modal_window::get_filename;

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct DataFrameSelect {
    dataframe_index: usize,
}

impl Default for DataFrameSelect {
    fn default() -> Self {
        Self { dataframe_index: 0 }
    }
}

impl DataFrameSelect {
    pub fn select(
        &mut self,
        ui: &mut egui::Ui,
        common_data: &common_data::CommonData,
    ) -> DataFrame {
        ui.horizontal(|ui| {
            if common_data.dataframes.len() == 0 {
                ui.label("Load DataFrame");
            } else {
                let (df_info, _) = common_data.dataframes.get(self.dataframe_index).unwrap();
                let fname = get_filename(df_info.filepath.as_str());
                ui.label("Select DataFrame");
                egui::ComboBox::from_label("")
                    .selected_text(format!("{}", fname))
                    .show_ui(ui, |ui| {
                        ui.style_mut().wrap = Some(false);
                        ui.set_min_width(60.0);
                        for (i, (df_info, df)) in common_data.dataframes.iter().enumerate() {
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
    }
}
