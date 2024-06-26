use crate::common_data::CommonData;
use crate::components::modal_window;
use eframe::egui;

use super::modal_window::get_filename;

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Clone)]
enum ExplorerTab {
    FILESYSTEM,
    DATAFRAME,
    ROSBAG,
}

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Explorer {
    explorer_tab: ExplorerTab,
    checked_map: std::collections::HashMap<String, bool>,
}

impl Default for Explorer {
    fn default() -> Self {
        Self {
            explorer_tab: ExplorerTab::FILESYSTEM,
            checked_map: std::collections::HashMap::new(),
        }
    }
}

impl Explorer {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        common_data_arc: std::sync::Arc<std::sync::Mutex<CommonData>>,
    ) {
        let common_data = common_data_arc.lock();
        if common_data.is_err() {
            return;
        }
        let mut common_data = common_data.unwrap();

        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.explorer_tab, ExplorerTab::FILESYSTEM, "Files");
            ui.selectable_value(&mut self.explorer_tab, ExplorerTab::DATAFRAME, "DataFrames");
            ui.selectable_value(&mut self.explorer_tab, ExplorerTab::ROSBAG, "Rosbags");
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

                let mut n_selected_dir = 0;
                let mut selected_dir = None;
                egui::ScrollArea::both().show(ui, |ui| -> Option<()> {
                    let mut update_list = false;
                    let promise = common_data.fs_list_promise.as_ref()?;
                    if promise.ready().is_none() && self.checked_map.len() != 0 {
                        log::info!("reset map!");
                        self.checked_map.clear();
                    }
                    let fs_list = promise.ready()?.as_ref().ok()?.clone();
                    if self.checked_map.len() == 0 {
                        for dirname in fs_list.directories.iter() {
                            self.checked_map.insert(dirname.clone(), false);
                        }
                        for filename in fs_list.files.iter() {
                            self.checked_map.insert(filename.clone(), false);
                        }
                    }

                    let mut fsvec = fs_list.files.clone();
                    fsvec.sort();

                    let nfp = std::path::Path::new(common_data.current_path.as_str());
                    let anc = nfp.clone();
                    let mut anc = anc.ancestors();
                    anc.next()?;
                    if let Some(anc_path) = anc.next() {
                        let mut b1 = false;
                        if ui.checkbox(&mut b1, "..").double_clicked() {
                            common_data.current_path = anc_path.to_string_lossy().into_owned();
                            update_list = true;
                        }
                    }

                    // for each directory entry
                    for dirname in fs_list.directories.iter() {
                        let c_box =
                            ui.checkbox(&mut self.checked_map.get_mut(dirname).unwrap(), dirname);
                        if c_box.double_clicked() {
                            let nfp = std::path::Path::new(common_data.current_path.as_str())
                                .join(dirname);
                            common_data.current_path = nfp.to_string_lossy().into_owned();
                            update_list = true;
                        }
                        if *self.checked_map.get(dirname).unwrap() {
                            // if selected
                            n_selected_dir += 1;
                            let nfp = std::path::Path::new(common_data.current_path.as_str())
                                .join(dirname);
                            selected_dir = Some(nfp.to_string_lossy().to_string().clone());
                        }
                    }
                    for filename in fsvec.iter() {
                        if ui
                            .checkbox(&mut self.checked_map.get_mut(filename).unwrap(), filename)
                            .double_clicked()
                        {
                            let nfp = std::path::Path::new(common_data.current_path.as_str())
                                .join(filename);
                            let fullpath = nfp.to_string_lossy().to_string();
                            common_data.modal_window_input_opt =
                                Some(modal_window::ModalWindowInput { filepath: fullpath });
                        }
                    }

                    if update_list {
                        common_data.fs_list_promise = Some(
                            common_data
                                .backend
                                .request_list(common_data.current_path.to_string()),
                        );
                    }
                    None
                });
                ui.separator();
                if ui.button("Load as ROSBAG2").clicked() {
                    if n_selected_dir == 1 {
                        common_data.load_rosbag2(selected_dir.unwrap());
                    }
                }
            }
            ExplorerTab::DATAFRAME => {
                egui::ScrollArea::both().show(ui, |ui| {
                    for (id, df_info) in common_data.latest_df_info_map.iter() {
                        ui.push_id(
                            format!("df_list_{}", df_info.clone().id.unwrap().id),
                            |ui| {
                                ui.collapsing(get_filename(&df_info.df_path), |ui| {
                                    egui::Grid::new("colors")
                                        .num_columns(2)
                                        .spacing([12.0, 8.0])
                                        .striped(true)
                                        .show(ui, |ui| {
                                            ui.label("ID");
                                            ui.label(format!("{}", id));
                                            ui.end_row();

                                            ui.label("FilePath");
                                            ui.label(format!("{}", df_info.df_path));
                                            ui.end_row();

                                            if let Some(df_opt) = common_data
                                                .required_dataframes
                                                .get(&(df_info.clone().id.unwrap().id as usize))
                                            {
                                                ui.label("Availability");
                                                ui.label(format!("{}", df_opt.is_some()));
                                                ui.end_row();

                                                if let Some(df) = df_opt {
                                                    ui.label("Shape");
                                                    ui.label(format!("{:?}", df.shape()));
                                                    ui.end_row();
                                                }
                                            }
                                        });
                                });
                            },
                        );
                    }
                });
            }
            ExplorerTab::ROSBAG => {}
        }
    }
}
