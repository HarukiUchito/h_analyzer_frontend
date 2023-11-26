use std::cmp::Ordering;

use eframe::egui::{self, Order};
use polars::prelude::*;

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Hash, Clone, Copy)]
pub enum DataFrameType {
    COMMA_SEP,
    NDEV,
    KITTI,
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Hash, Clone, Copy)]
pub enum LoadState {
    OPEN_MODAL_WINDOW,
    LOAD_NOW,
    LOADING,
    LOADED,
}

#[derive(serde::Deserialize, serde::Serialize, Hash, Clone)]
pub struct DataFrameInfo {
    pub filepath: String,
    pub df_type: DataFrameType,
    pub load_state: LoadState,
}

impl DataFrameInfo {
    pub fn new(fpath: String) -> DataFrameInfo {
        DataFrameInfo {
            filepath: fpath,
            df_type: DataFrameType::NDEV,
            load_state: LoadState::OPEN_MODAL_WINDOW,
        }
    }
}

impl Ord for DataFrameInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.filepath.cmp(&other.filepath)
    }
}

impl PartialOrd for DataFrameInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for DataFrameInfo {
    fn eq(&self, other: &Self) -> bool {
        (self.filepath.clone(), self.df_type, self.load_state)
            == (other.filepath.clone(), other.df_type, other.load_state)
    }
}

impl Eq for DataFrameInfo {}

pub fn get_filename(fullpath: &str) -> String {
    let pathv = std::path::Path::new(fullpath);
    let name = pathv.file_name().unwrap().to_string_lossy().to_string();
    name
}

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ModalWindow {}

impl Default for ModalWindow {
    fn default() -> Self {
        Self {}
    }
}

impl ModalWindow {
    pub fn show(&mut self, ctx: &egui::Context, df_info: &mut DataFrameInfo) {
        egui::Window::new("modal")
            //                .open(&mut self.modal_window_open)
            .anchor(egui::Align2::CENTER_TOP, egui::Vec2::new(0.0, 100.0))
            .show(ctx, |ui| {
                let name = get_filename(df_info.filepath.as_str());
                ui.label(name.clone());
                ui.label("dataframe type");
                if ui
                    .add(egui::RadioButton::new(
                        df_info.df_type == DataFrameType::COMMA_SEP,
                        "COMMA_SEP",
                    ))
                    .clicked()
                {
                    df_info.df_type = DataFrameType::COMMA_SEP;
                }
                if ui
                    .add(egui::RadioButton::new(
                        df_info.df_type == DataFrameType::NDEV,
                        "NDEV",
                    ))
                    .clicked()
                {
                    df_info.df_type = DataFrameType::NDEV;
                }
                if ui
                    .add(egui::RadioButton::new(
                        df_info.df_type == DataFrameType::KITTI,
                        "KITTI",
                    ))
                    .clicked()
                {
                    df_info.df_type = DataFrameType::KITTI;
                }
                if ui.button("Load File").clicked() {
                    df_info.load_state = LoadState::LOAD_NOW;
                }
            });
    }
}
