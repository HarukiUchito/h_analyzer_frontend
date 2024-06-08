use super::dataframe_table;
use eframe::egui::{self};
use polars::prelude::*;

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct ModalWindowInput {
    pub filepath: String,
}

pub fn get_filename(fullpath: &str) -> String {
    let pathv = std::path::Path::new(fullpath);
    let name = pathv.file_name().unwrap().to_string_lossy().to_string();
    name
}

#[derive(PartialEq, serde::Deserialize, serde::Serialize, Clone)]
pub enum ModalWindowAction {
    Nothing,
    Preview,
    Load,
    Cancel,
}

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ModalWindow {
    done: bool,
    load_option: h_analyzer_data::grpc_fs::DataFrameLoadOption,
}

impl Default for ModalWindow {
    fn default() -> Self {
        Self {
            done: false,
            load_option: h_analyzer_data::grpc_fs::DataFrameLoadOption {
                source_type: h_analyzer_data::grpc_fs::DataFrameSourceType::Csv.into(),
                has_header: false,
                skip_row_num_before_header: 0,
                skip_row_num_after_header: 0,
                delimiter: ",".to_string(),
                updated: false,
            },
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct ModalWindowOutput {
    pub action: ModalWindowAction,
    pub filepath: String,
    pub load_option: h_analyzer_data::grpc_fs::DataFrameLoadOption,
}

impl ModalWindow {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        filepath: String,
        df_opt: &Option<DataFrame>,
        still_open: &mut bool,
        load_now: &mut bool,
    ) -> ModalWindowOutput {
        let mut action = ModalWindowAction::Nothing;
        *still_open = true;
        egui::Window::new("File Load Options")
            //                .open(&mut self.modal_window_open)
            .anchor(egui::Align2::CENTER_TOP, egui::Vec2::new(0.0, 100.0))
            .show(ctx, |ui| {
                egui::Grid::new("settings").show(ui, |ui| {
                    let name = get_filename(filepath.as_str());
                    ui.horizontal(|ui| {
                        ui.label("Filename:");
                        ui.label(name.clone());
                    });
                    ui.end_row();

                    ui.horizontal(|ui| {
                        ui.label("Data Source Type:");
                        ui.selectable_value(
                            &mut self.load_option.source_type,
                            h_analyzer_data::grpc_fs::DataFrameSourceType::Csv as i32,
                            "CSV",
                        );
                        ui.selectable_value(
                            &mut self.load_option.source_type,
                            h_analyzer_data::grpc_fs::DataFrameSourceType::Rosbag2 as i32,
                            "ROSBAG2",
                        );
                    });
                    ui.end_row();

                    if self.load_option.source_type
                        == h_analyzer_data::grpc_fs::DataFrameSourceType::Csv as i32
                    {
                        ui.horizontal(|ui| {
                            ui.label("Load options:");
                            ui.horizontal(|ui| {
                                ui.label("numer of rows to skip before header");
                                ui.add(egui::DragValue::new(
                                    &mut self.load_option.skip_row_num_before_header,
                                ));
                            });
                        });
                        ui.end_row();

                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.load_option.has_header, "has_header");
                        });
                        ui.end_row();

                        ui.horizontal(|ui| {
                            ui.label("numer of rows to skip after header");
                            ui.add(egui::DragValue::new(
                                &mut self.load_option.skip_row_num_after_header,
                            ));
                        });
                        ui.end_row();
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Load File").clicked() {
                            *still_open = false;
                            *load_now = true;
                            action = ModalWindowAction::Load;
                        }
                        if ui.button("Preview").clicked() {
                            *load_now = true;
                            action = ModalWindowAction::Preview;
                        }
                        if ui.button("Cancel").clicked() {
                            *still_open = false;
                            action = ModalWindowAction::Cancel;
                        }
                    });
                    ui.end_row();
                    if let Some(df) = df_opt {
                        dataframe_table::show_dataframe_table(ui, df);
                    }
                });
            });
        ModalWindowOutput {
            action: action,
            filepath: filepath,
            load_option: self.load_option.clone(),
        }
    }
}
