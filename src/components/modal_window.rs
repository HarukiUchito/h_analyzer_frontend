use super::dataframe_table;
use eframe::egui::{self};
use polars::prelude::*;

#[derive(
    strum_macros::Display, serde::Deserialize, serde::Serialize, PartialEq, Hash, Clone, Copy,
)]
pub enum LoadState {
    OpenModalWindow,
    LoadNow,
    LOADING,
    LOADED,
    FAILED,
    CANCELED,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct DataFrameInfo {
    pub filepath: String,
    pub load_option: h_analyzer_data::grpc_fs::DataFrameLoadOption,
    pub load_state: LoadState,
}

impl DataFrameInfo {
    pub fn new(fpath: String) -> DataFrameInfo {
        DataFrameInfo {
            filepath: fpath,
            load_option: h_analyzer_data::grpc_fs::DataFrameLoadOption {
                has_header: false,
                skip_row_num_before_header: 0,
                skip_row_num_after_header: 0,
                delimiter: ",".to_string(),
            },
            load_state: LoadState::OpenModalWindow,
        }
    }
}

pub fn get_filename(fullpath: &str) -> String {
    let pathv = std::path::Path::new(fullpath);
    let name = pathv.file_name().unwrap().to_string_lossy().to_string();
    name
}

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ModalWindow {
    done: bool,
}

impl Default for ModalWindow {
    fn default() -> Self {
        Self { done: false }
    }
}

impl ModalWindow {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        df_info: &mut DataFrameInfo,
        df_opt: &Option<DataFrame>,
        still_open: &mut bool,
    ) {
        *still_open = true;
        egui::Window::new("modal")
            //                .open(&mut self.modal_window_open)
            .anchor(egui::Align2::CENTER_TOP, egui::Vec2::new(0.0, 100.0))
            .show(ctx, |ui| {
                let name = get_filename(df_info.filepath.as_str());
                ui.label(name.clone());
                ui.label("Load options");
                ui.horizontal(|ui| {
                    ui.label("numer of rows to skip before header");
                    ui.add(egui::DragValue::new(
                        &mut df_info.load_option.skip_row_num_before_header,
                    ));
                });
                ui.checkbox(&mut df_info.load_option.has_header, "has_header");
                ui.horizontal(|ui| {
                    ui.label("numer of rows to skip after header");
                    ui.add(egui::DragValue::new(
                        &mut df_info.load_option.skip_row_num_after_header,
                    ));
                });
                if ui.button("Load File").clicked() {
                    df_info.load_state = LoadState::LoadNow;
                    *still_open = false;
                }
                if ui.button("Preview").clicked() {
                    df_info.load_state = LoadState::LoadNow;
                }
                if ui.button("Cancel").clicked() {
                    df_info.load_state = LoadState::CANCELED;
                    *still_open = false;
                }
                if let Some(df) = df_opt {
                    dataframe_table::show_dataframe_table(ui, df);
                }
            });
    }
}
