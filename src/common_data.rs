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

    pub dataframes: HashMap<String, (modal_window::DataFrameInfo, Option<DataFrame>)>,
    pub current_path: String,
    pub default_path: String,

    #[serde(skip)]
    pub world_list_promise:
        Option<Promise<Result<grpc_data_transfer::WorldMetadataList, tonic::Status>>>,

    #[serde(skip)]
    pub world_frame_promise: Option<Promise<Result<h_analyzer_data::WorldFrame, tonic::Status>>>,
    pub world: h_analyzer_data::World,

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
    pub hello_promise: Option<(usize, Promise<Result<DataFrame, tonic::Status>>)>,
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
        let w_promise = backend.get_world_frame("slam".to_string(), 0.0);
        let wl_promise = backend.get_world_list();
        Self {
            backend: backend,
            dataframes: HashMap::new(),
            current_path: path.clone(),
            default_path: path.clone(),

            world_list_promise: Some(wl_promise),
            world_frame_promise: Some(w_promise),
            world: h_analyzer_data::World::new(),

            series_list_req_time: web_time::Instant::now(),
            sl_time_history: std::collections::VecDeque::new(),

            init_df_list_promise: Some(init_df_list_promise),
            df_to_be_loaded_queue: VecDeque::new(),
            save_df_list_promise: None,
            fs_list_promise: Some(fs_list_promise),
            hello_promise: None,
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
                df_type: df_info.df_type as i32,
            });
        }
        self.save_df_list_promise = Some(self.backend.save_df_list(dfi_list));
    }

    pub fn update(&mut self, selected_world_name: Option<String>) {
        if let Some(wf_promise) = &self.world_frame_promise {
            if let Some(wf) = wf_promise.ready() {
                if let Ok(wf) = wf {
                    log::info!("world frame {}", wf);
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
                let selected_world_name = if let Some(name) = selected_world_name {
                    name
                } else {
                    "slam".to_string()
                };
                self.series_list_req_time = web_time::Instant::now();
                self.world_frame_promise =
                    Some(self.backend.get_world_frame(selected_world_name, 0.0));
            }
        }

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
            df_info.df_type = match df_to_be_loaded.df_type {
                0 => modal_window::DataFrameType::CommaSep,
                1 => modal_window::DataFrameType::NDEV,
                2 => modal_window::DataFrameType::KITTI,
                _ => unimplemented!(),
            };
            df_info.load_state = modal_window::LoadState::LoadNow;
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

        let df = (|| {
            if let Some((idx, result)) = &self.hello_promise {
                if let Some(result) = result.ready() {
                    if let Ok(result) = result {
                        if let Some(entry) = self.dataframes.get_mut(&idx.to_string()) {
                            log::info!("df stored {:?}", result.shape());
                            log::info!("{:?}", result);
                            entry.1 = Some(result.clone());
                            log::info!("{:?}", entry.1.clone().unwrap_or_default());
                        }
                        return result.clone();
                    }
                }
            }
            DataFrame::default()
        })();
        if !df.is_empty() {
            self.hello_promise = None;
        }

        for (idx, (_, (df_info, _))) in self.dataframes.iter_mut().enumerate() {
            if df_info.load_state == LoadState::LoadNow && self.hello_promise.is_none() {
                self.hello_promise = Some((
                    idx,
                    self.backend
                        .load_df_request(df_info.filepath.clone(), df_info.df_type),
                ));
                df_info.load_state = LoadState::LOADING;
            }
        }
    }
}
