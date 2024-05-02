use std::collections::VecDeque;

use crate::backend_talk::{self, grpc_data_transfer, grpc_fs};
use crate::components::modal_window::{self, LoadState};
use crate::unwrap_or_continue;
use polars::prelude::*;
use poll_promise::Promise;
use std::collections::HashMap;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct CommonData {
    #[serde(skip)]
    pub backend: backend_talk::BackendTalk,

    pub modal_window_df_key: Option<String>,
    pub dataframes: HashMap<String, (modal_window::DataFrameInfo, Option<DataFrame>)>,
    pub current_path: String,
    pub default_path: String,

    #[serde(skip)]
    pub world_list_promise:
        Option<Promise<Result<grpc_data_transfer::WorldMetadataList, tonic::Status>>>,

    #[serde(skip)]
    pub world_frame_promise: Option<Promise<Result<h_analyzer_data::WorldFrame, tonic::Status>>>,
    #[serde(skip)]
    pub world: h_analyzer_data::World,
    #[serde(skip)]
    pub world_name: String,
    pub world_playing: bool,

    #[serde(skip)]
    series_list_req_time: web_time::Instant,
    pub sl_time_history: std::collections::VecDeque<f64>,

    #[serde(skip)]
    pub init_df_list_promise:
        Option<Promise<Result<backend_talk::grpc_fs::DataFrameInfoList, tonic::Status>>>,
    #[serde(skip)]
    df_to_be_loaded_queue: VecDeque<backend_talk::grpc_fs::DataFrameInfo>,

    #[serde(skip)]
    pub save_df_list_promise: Option<Promise<Result<backend_talk::grpc_fs::Empty, tonic::Status>>>,
    #[serde(skip)]
    pub fs_list_promise:
        Option<Promise<Result<backend_talk::grpc_fs::ListResponse, tonic::Status>>>,
    #[serde(skip)]
    pub load_df_promise: Option<(String, Promise<Result<DataFrame, tonic::Status>>)>,
    #[serde(skip)]
    pub d_path_promise: Option<Promise<Result<backend_talk::grpc_fs::PathMessage, tonic::Status>>>,
}

impl Default for CommonData {
    fn default() -> Self {
        let path = "/".to_string();
        let backend = backend_talk::BackendTalk::default();
        let init_df_list_promise = backend.request_initial_df_list();
        let fs_list_promise = backend.request_list(path.clone());
        let d_path_promise = backend.request_default_path();
        let wl_promise = backend.get_world_list();
        Self {
            backend: backend,
            modal_window_df_key: None,
            dataframes: HashMap::new(),
            current_path: path.clone(),
            default_path: path.clone(),

            world_list_promise: Some(wl_promise),
            world_frame_promise: None,
            world: h_analyzer_data::World::new(),
            world_name: "".to_string(),
            world_playing: true,

            series_list_req_time: web_time::Instant::now(),
            sl_time_history: std::collections::VecDeque::new(),

            init_df_list_promise: Some(init_df_list_promise),
            df_to_be_loaded_queue: VecDeque::new(),
            save_df_list_promise: None,
            fs_list_promise: Some(fs_list_promise),
            load_df_promise: None,
            d_path_promise: Some(d_path_promise),
        }
    }
}

impl CommonData {
    pub fn update_world_list(&mut self) {
        self.world_list_promise = Some(self.backend.get_world_list());
    }

    pub fn get_world_list(&mut self) -> Option<grpc_data_transfer::WorldMetadataList> {
        if let Some(world_list_promise) = &self.world_list_promise {
            if let Ok(world_list) = world_list_promise.ready()? {
                let ret = world_list.clone();
                return Some(ret);
            }
        }
        None
    }

    pub fn save_df_list(&mut self) {
        let mut dfi_list = Vec::new();
        for (_, (df_info, _)) in self.dataframes.iter() {
            dfi_list.push(grpc_fs::DataFrameInfo {
                df_path: df_info.filepath.clone(),
                load_option: Some(df_info.load_option.clone()),
            });
        }
        self.save_df_list_promise = Some(self.backend.save_df_list(dfi_list));
    }

    pub fn update(&mut self, selected_world_name: Option<String>) {
        // world frame update
        let selected_world_name = if let Some(name) = selected_world_name {
            name
        } else {
            "slam".to_string()
        };
        if selected_world_name != self.world_name {
            self.world.reset();
        }
        if self.world_playing {
            self.world.next();
        }
        if self.world_frame_promise.is_none() {
            let mut w_f_num = 0;
            if let Some(world_list) = self.get_world_list() {
                for world_meta in world_list.list.iter() {
                    let wname = world_meta.id.clone().unwrap().id;
                    if wname == selected_world_name {
                        w_f_num = world_meta.total_frame_num;
                    }
                }
            }
            log::debug!(
                "total frame num: {}, current: {}",
                w_f_num,
                self.world.history.len()
            );
            if self.world.history.len() < w_f_num as usize {
                self.series_list_req_time = web_time::Instant::now();
                self.world_frame_promise =
                    Some(self.backend.get_world_frame(
                        selected_world_name.clone(),
                        self.world.history.len() as u32,
                    ));
                self.world_name = selected_world_name.clone();
            }
        }
        if let Some(wf_promise) = &self.world_frame_promise {
            if let Some(wf) = wf_promise.ready() {
                if let Ok(wf) = wf {
                    //log::info!("world frame {}", wf);
                    if let Some(lwf) = self.world.history.last() {
                        if lwf.frame_index > wf.frame_index {
                            // loop detected
                            self.world.reset();
                        }
                    }
                    self.world.history.push(wf.clone());

                    let et = self.series_list_req_time.elapsed().as_nanos() as f64;
                    self.sl_time_history.push_back(et * 1e-9);
                    if self.sl_time_history.len() > 10 {
                        self.sl_time_history.pop_front();
                    }
                }
                self.world_frame_promise = None;
            }
        }

        // dataframe list update
        if let Some(init_df_list_promise) = &self.init_df_list_promise {
            if let Some(init_df_list) = init_df_list_promise.ready() {
                if let Ok(init_df_list) = init_df_list {
                    for df_info in init_df_list.list.iter() {
                        self.df_to_be_loaded_queue.push_back(df_info.clone());
                    }
                }
                self.init_df_list_promise = None;
            }
        }

        if let Some(df_to_be_loaded) = self.df_to_be_loaded_queue.pop_front() {
            let mut df_info = modal_window::DataFrameInfo::new(df_to_be_loaded.df_path);
            df_info.load_option = df_to_be_loaded.load_option.unwrap();
            df_info.load_state = modal_window::LoadState::LoadNow;
            log::info!("enqueue {} {}", self.dataframes.len(), df_info.filepath);
            self.dataframes
                .insert(self.dataframes.len().to_string(), (df_info, None));
        }

        // take should not be used in following line because taken promise must be put it back if it's not ready
        if let Some(d_path_promise) = &self.d_path_promise {
            if let Some(d_path) = d_path_promise.ready() {
                log::info!("d_path_promise: {:?}", d_path);
                if let Ok(d_path) = d_path.as_ref() {
                    let d_path = d_path.path.clone();
                    self.current_path = d_path.clone();
                    self.default_path = d_path.clone();
                    self.fs_list_promise = Some(self.backend.request_list(d_path.clone()));
                }

                self.d_path_promise = None;
            }
        }

        let recieved = (|| {
            if let Some((idx, result)) = &self.load_df_promise {
                if let Some(entry) = self.dataframes.get_mut(&idx.to_string()) {
                    if let Some(result) = result.ready() {
                        log::info!("done");
                        if let Ok(result) = result {
                            //log::info!("df stored {:?}", result.shape());
                            //log::info!("{:?}", result);
                            entry.0.load_state = LoadState::LOADED;
                            entry.1 = Some(result.clone());
                            log::info!("assigned to {}", idx);
                            //log::info!("{:?}", entry.1.clone().unwrap_or_default());
                        } else {
                            entry.0.load_state = LoadState::FAILED;
                            entry.1 = None;
                        }
                        return true;
                    }
                }
            }
            false
        })();
        if recieved {
            self.load_df_promise = None;
        }

        let mut remove_list = Vec::new();
        for (idx, (df_info, _)) in self.dataframes.iter_mut() {
            if df_info.load_state == LoadState::CANCELED {
                remove_list.push(idx.clone());
            }
            if df_info.load_state == LoadState::LoadNow && self.load_df_promise.is_none() {
                self.load_df_promise = Some((
                    idx.to_string(),
                    self.backend
                        .load_df_request(df_info.filepath.clone(), df_info.load_option.clone()),
                ));
                df_info.load_state = LoadState::LOADING;
                log::info!("evoke {} {}", idx, df_info.filepath);
            }
        }
        for rm_key in remove_list.iter() {
            self.dataframes.remove(rm_key);
        }
    }
}
