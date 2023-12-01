use std::collections::VecDeque;

use crate::backend_talk::{self, grpc_fs};
use crate::components::modal_window::{self, LoadState};
use polars::prelude::*;
use poll_promise::Promise;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct CommonData {
    pub dataframes: Vec<(modal_window::DataFrameInfo, Option<DataFrame>)>,
    pub current_path: String,
    pub default_path: String,

    #[serde(skip)]
    pub backend: backend_talk::BackendTalk,

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
        Self {
            backend: backend,
            dataframes: Vec::new(),
            current_path: path.clone(),
            default_path: path.clone(),
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
    pub fn save_df_list(&mut self) {
        let mut dfi_list = Vec::new();
        for (df_info, _) in self.dataframes.iter() {
            dfi_list.push(grpc_fs::DataFrameInfo {
                df_path: df_info.filepath.clone(),
                df_type: df_info.df_type as i32,
            });
        }
        self.save_df_list_promise = Some(self.backend.save_df_list(dfi_list));
    }

    pub fn update(&mut self) {
        if let Some(init_df_list_promise) = &self.init_df_list_promise {
            if let Some(init_df_list) = init_df_list_promise.ready() {
                if let Ok(init_df_list) = init_df_list {
                    for df_info in init_df_list.list.iter() {
                        self.df_to_be_loaded_queue.push_back(df_info.clone());
                    }
                    self.init_df_list_promise = None;
                }
            }
        }

        if let Some(df_to_be_loaded) = self.df_to_be_loaded_queue.pop_front() {
            let mut df_info = modal_window::DataFrameInfo::new(df_to_be_loaded.df_path);
            df_info.df_type = match df_to_be_loaded.df_type {
                0 => modal_window::DataFrameType::COMMA_SEP,
                1 => modal_window::DataFrameType::NDEV,
                2 => modal_window::DataFrameType::KITTI,
                _ => unimplemented!(),
            };
            df_info.load_state = modal_window::LoadState::LOAD_NOW;
            self.dataframes.push((df_info, None));
        }

        if let Some(d_path_promise) = &self.d_path_promise {
            if let Some(d_path) = d_path_promise.ready() {
                log::info!("d_path_promise: {:?}", d_path);
                let d_path = d_path.as_ref().unwrap().path.clone();
                self.current_path = d_path.clone();
                self.default_path = d_path.clone();
                self.fs_list_promise = Some(self.backend.request_list(d_path.clone()));
                self.d_path_promise = None;
            }
        }

        let df = (|| {
            if let Some((idx, result)) = &self.hello_promise {
                if let Some(result) = result.ready() {
                    if let Ok(result) = result {
                        if let Some(entry) = self.dataframes.get_mut(*idx) {
                            entry.1 = Some(result.clone());
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

        for (idx, (df_info, _)) in self.dataframes.iter_mut().enumerate() {
            if df_info.load_state == LoadState::LOAD_NOW && self.hello_promise.is_none() {
                let name = modal_window::get_filename(df_info.filepath.as_str());
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
