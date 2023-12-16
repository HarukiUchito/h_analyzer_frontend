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

    pub realtime_dataframes: HashMap<String, DataFrame>,
    #[serde(skip)]
    pub realtime_promises: HashMap<
        grpc_data_transfer::SeriesId,
        Option<Promise<Result<backend_talk::ResponseType, tonic::Status>>>,
    >,
    #[serde(skip)]
    pub series_list_promise:
        Option<Promise<Result<grpc_data_transfer::SeriesMetadataList, tonic::Status>>>,

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
            dataframes: HashMap::new(),
            current_path: path.clone(),
            default_path: path.clone(),

            realtime_dataframes: HashMap::new(),
            realtime_promises: HashMap::new(),
            series_list_promise: None,

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
        for (_, (df_info, _)) in self.dataframes.iter() {
            dfi_list.push(grpc_fs::DataFrameInfo {
                df_path: df_info.filepath.clone(),
                df_type: df_info.df_type as i32,
            });
        }
        self.save_df_list_promise = Some(self.backend.save_df_list(dfi_list));
    }

    fn update_realtime_dataframe(&mut self) -> Option<()> {
        if self.series_list_promise.is_none() {
            self.series_list_promise = Some(self.backend.get_series_list());
        }

        let series_list = self.series_list_promise.as_ref()?.ready()?.as_ref().ok()?;
        log::info!("{:?}", series_list);
        for metadata in series_list.list.iter() {
            let df_id = unwrap_or_continue!(metadata.clone().id);
            let df_id_str = df_id.id.clone();
            match self.realtime_promises.get_mut(&df_id) {
                Some(r_promise_opt) => {
                    if r_promise_opt.is_none() {
                        *r_promise_opt = Some(match metadata.element_type {
                            0 => self.backend.poll_point_2d_queue(df_id.clone()),
                            1 => self.backend.poll_pose_2d_queue(df_id.clone()),
                            _ => panic!("unexpected element type"),
                        });
                    }
                    let new_poll = unwrap_or_continue!(r_promise_opt).ready()?;
                    if let Ok(new_poll) = new_poll {
                        let mut xs = Vec::new();
                        let mut ys = Vec::new();
                        let mut ts = Vec::new();
                        let mut command = None;
                        match new_poll {
                            backend_talk::ResponseType::Point2D(points) => {
                                command = Some(points.command);
                                for p in points.points.iter() {
                                    xs.push(p.x);
                                    ys.push(p.y);
                                }
                            }
                            backend_talk::ResponseType::Pose2D(poses) => {
                                command = Some(poses.command);
                                for p in poses.poses.iter() {
                                    xs.push(p.position.as_ref().unwrap().x);
                                    ys.push(p.position.as_ref().unwrap().y);
                                    ts.push(p.theta);
                                }
                            }
                        }
                        if let Some(inner_df) = self.realtime_dataframes.get_mut(&df_id_str) {
                            if let Some(command) = command {
                                match command {
                                    1 => {
                                        *inner_df = inner_df.clear();
                                    }
                                    _ => {
                                        let new_df = match metadata.element_type {
                                            0 => df!("x[m]" => &xs,
                                    "y[m]" => &ys)
                                            .unwrap(),
                                            1 => 
                                                df!("x[m]" => &xs, "y[m]" => &ys, "theta[rad]" => &ts).unwrap(),
                                            _ => panic!("unexpected element type"),
                                        };
                                        *inner_df = inner_df.vstack(&new_df).unwrap();
                                    }
                                }
                            }
                        }
                    }
                    *r_promise_opt = None;
                }
                None => {
                    let empty_df = match metadata.element_type {
                        0 => df!("x[m]" => &([] as [f64; 0]),
                    "y[m]" => &([] as [f64; 0]))
                        .ok()?,
                        1 => df!("x[m]" => &([] as [f64; 0]),
                        "y[m]" => &([] as [f64; 0]), "theta[rad]" => &([] as [f64; 0]))
                        .ok()?,
                        _ => panic!("unexpected element type"),
                    };
                    self.realtime_dataframes.insert(df_id_str, empty_df);
                    self.realtime_promises.insert(df_id.clone(), None);
                }
            }
        }
        self.series_list_promise = None;
        None
    }

    pub fn update(&mut self) {
        self.update_realtime_dataframe();

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
