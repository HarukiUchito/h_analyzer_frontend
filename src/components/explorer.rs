use crate::common_data::CommonData;
use crate::components::modal_window;
use eframe::egui;
use polars::prelude::*;

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Clone)]
enum ExplorerTab {
    FILESYSTEM,
    DATAFRAME,
}

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Explorer {
    explorer_tab: ExplorerTab,
}

impl Default for Explorer {
    fn default() -> Self {
        Self {
            explorer_tab: ExplorerTab::FILESYSTEM,
        }
    }
}

impl Explorer {
    pub fn show(&mut self, ui: &mut egui::Ui, common_data: &mut CommonData) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.explorer_tab, ExplorerTab::FILESYSTEM, "Files");
            ui.selectable_value(&mut self.explorer_tab, ExplorerTab::DATAFRAME, "DataFrames");
        });

        ui.separator();
        match self.explorer_tab {
            ExplorerTab::FILESYSTEM => {
                ui.label(common_data.current_path.as_str());
                if ui.button("refresh").clicked() {
                    common_data.fs_list_promise = Some(
                        common_data
                            .backend
                            .request_list(common_data.current_path.clone()),
                    );
                }

                ui.separator();

                egui::ScrollArea::both().show(ui, |ui| {
                    let mut update_list = false;
                    if let Some(fs_list) = &common_data.fs_list_promise {
                        if let Some(fs_list) = fs_list.ready() {
                            if let Ok(fs_list) = fs_list {
                                let mut fsvec = fs_list.files.clone();
                                fsvec.sort();
                                let mut b1 = false;
                                let dirname = "..";
                                if ui.checkbox(&mut b1, dirname).double_clicked() {
                                    let nfp =
                                        std::path::Path::new(common_data.current_path.as_str())
                                            .join(dirname);
                                    common_data.current_path = nfp.to_string_lossy().into_owned();
                                    update_list = true;
                                }
                                for dirname in fs_list.directories.iter() {
                                    if ui.checkbox(&mut b1, dirname).double_clicked() {
                                        let nfp =
                                            std::path::Path::new(common_data.current_path.as_str())
                                                .join(dirname);
                                        common_data.current_path =
                                            nfp.to_string_lossy().into_owned();
                                        update_list = true;
                                    }
                                }
                                for filename in fsvec.iter() {
                                    let mut b1 = false;
                                    if ui.checkbox(&mut b1, filename).double_clicked() {
                                        common_data.modal_window_open = true;
                                        let nfp =
                                            std::path::Path::new(common_data.current_path.as_str())
                                                .join(filename);
                                        common_data.dataframe_info =
                                            Some(modal_window::DataFrameInfo::new(
                                                nfp.to_string_lossy().to_string(),
                                            ));
                                    }
                                }
                            }
                        }
                    }
                    if update_list {
                        common_data.fs_list_promise = Some(
                            common_data
                                .backend
                                .request_list(common_data.current_path.to_string()),
                        );
                    }
                });
            }
            ExplorerTab::DATAFRAME => {
                for (name, _) in common_data.dataframes.iter() {
                    let mut checkd = false;
                    ui.checkbox(&mut checkd, name.clone());
                }
            }
        }
    }
}
