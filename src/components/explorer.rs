use crate::common_data::CommonData;
use crate::components::modal_window;
use eframe::egui;

use super::modal_window::get_filename;

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Clone)]
enum ExplorerTab {
    FILESYSTEM,
    DATAFRAME,
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

                    for dirname in fs_list.directories.iter() {
                        if ui
                            .checkbox(&mut self.checked_map.get_mut(dirname).unwrap(), dirname)
                            .double_clicked()
                        {
                            let nfp = std::path::Path::new(common_data.current_path.as_str())
                                .join(dirname);
                            common_data.current_path = nfp.to_string_lossy().into_owned();
                            update_list = true;
                        }
                    }
                    for filename in fsvec.iter() {
                        if ui
                            .checkbox(&mut self.checked_map.get_mut(filename).unwrap(), filename)
                            .double_clicked()
                        {
                            let nfp = std::path::Path::new(common_data.current_path.as_str())
                                .join(filename);
                            let key = common_data.dataframes.len().to_string().clone();
                            let fullpath = nfp.to_string_lossy().to_string();
                            common_data.dataframes.insert(
                                key.clone(),
                                (modal_window::DataFrameInfo::new(fullpath), None),
                            );
                            common_data.modal_window_df_key = Some(key);
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
                ui.button("Load as ROSBAG2");
            }
            ExplorerTab::DATAFRAME => {
                for (_, (df_info, df_opt)) in common_data.dataframes.iter() {
                    let mut checkd = false;
                    let name = get_filename(&df_info.filepath.as_str());
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut checkd, name.clone());
                        ui.label(df_info.load_state.to_string());
                        if let Some(df) = df_opt {
                            ui.label(format!("shape {:?}", df.shape()));
                        }
                    });
                    ui.label(df_info.filepath.to_string());
                }
            }
        }
    }
}
