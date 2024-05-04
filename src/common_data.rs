use std::borrow::BorrowMut;
use std::collections::VecDeque;

use crate::backend_talk::{self, grpc_data_transfer, grpc_fs};
use crate::components::modal_window;
use crate::unwrap_or_continue;
use polars::prelude::*;
use poll_promise::Promise;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct CommonData {
    #[serde(skip)]
    pub backend: backend_talk::BackendTalk,

    pub update_df_list: bool,
    pub required_dataframes: std::collections::HashMap<usize, Option<DataFrame>>,
    pub latest_df_info_map:
        std::collections::HashMap<usize, h_analyzer_data::grpc_fs::DataFrameInfo>,
    pub just_added_df_id_opt: Option<usize>,

    #[serde(skip)]
    pub get_df_list_promise:
        Option<Promise<Result<h_analyzer_data::grpc_fs::DataFrameInfoList, tonic::Status>>>,
    #[serde(skip)]
    pub get_df_promise: Option<(usize, Promise<Result<DataFrame, tonic::Status>>)>,
    #[serde(skip)]
    pub get_df_from_file_promise: Option<Promise<Result<usize, tonic::Status>>>,

    pub modal_window_input_opt: Option<modal_window::ModalWindowInput>,

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
        let fs_list_promise = backend.request_list(path.clone());
        let d_path_promise = backend.request_default_path();
        let wl_promise = backend.get_world_list();
        Self {
            backend: backend,

            update_df_list: true,
            required_dataframes: std::collections::HashMap::new(),
            get_df_list_promise: None,
            get_df_promise: None,
            get_df_from_file_promise: None,
            latest_df_info_map: std::collections::HashMap::new(),
            just_added_df_id_opt: None,

            modal_window_input_opt: None,
            current_path: path.clone(),
            default_path: path.clone(),

            world_list_promise: Some(wl_promise),
            world_frame_promise: None,
            world: h_analyzer_data::World::new(),
            world_name: "".to_string(),
            world_playing: true,

            series_list_req_time: web_time::Instant::now(),
            sl_time_history: std::collections::VecDeque::new(),

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
        //self.save_df_list_promise = Some(self.backend.save_df_list(dfi_list));
    }

    pub fn remove_preview_data_frame(&mut self) {
        if let Some(df_id) = self.just_added_df_id_opt {
            let _ = self
                .backend
                .remove_df_request(h_analyzer_data::grpc_fs::DataFrameId { id: df_id as u32 });
            self.just_added_df_id_opt = None;
            self.update_df_list = true;
        }
    }

    pub fn load_data_frame_from_file(
        &mut self,
        filepath: &String,
        load_option: &h_analyzer_data::grpc_fs::DataFrameLoadOption,
    ) {
        self.get_df_from_file_promise = Some(
            self.backend
                .load_df_from_file_request(filepath.clone(), load_option.clone()),
        );
        self.update_df_list = true;
    }

    pub fn request_df_transmission(&mut self, df_id: usize) {
        let rdf = self.required_dataframes.borrow_mut();
        if rdf.get(&df_id).is_none() && !rdf.contains_key(&df_id) {
            rdf.insert(df_id, None);
        }
    }

    pub fn get_just_loaded_data_frame(&mut self) -> Option<DataFrame> {
        if let Ok(id) = self.get_df_from_file_promise.as_ref()?.ready()? {
            self.just_added_df_id_opt = Some(id.clone());
            self.get_df_from_file_promise = None;
        }
        if let Some(just_added_df_id) = self.just_added_df_id_opt {
            self.request_df_transmission(just_added_df_id);
            self.required_dataframes.get(&just_added_df_id)?.clone()
        } else {
            None
        }
    }

    pub fn update(&mut self, selected_world_name: Option<String>) {
        // retrieve dataframe list update if needed
        if self.update_df_list {
            if self.get_df_list_promise.is_none() {
                self.get_df_list_promise = Some(self.backend.request_get_df_list());
            }
            if let Some(get_df_list) = &self.get_df_list_promise {
                if let Some(get_df_list) = get_df_list.ready() {
                    if let Ok(latest_df_list) = get_df_list {
                        self.latest_df_info_map.clear();
                        for df_info in latest_df_list.list.iter() {
                            self.latest_df_info_map
                                .insert(df_info.clone().id.unwrap().id as usize, df_info.clone());
                        }
                    }
                    self.update_df_list = false;
                    self.get_df_list_promise = None;
                }
            }
        }

        // request sending required dataframe from backend
        for required_df in self.required_dataframes.iter() {
            let id = *required_df.0;
            if required_df.1.is_none() && self.get_df_promise.is_none() {
                self.get_df_promise = Some((
                    id,
                    self.backend
                        .get_df_request(h_analyzer_data::grpc_fs::DataFrameId { id: id as u32 }),
                ));
            }
        }

        // check if the dataframe request has completed
        if let Some(get_df_promise) = &self.get_df_promise {
            let requested_df_id = get_df_promise.0;
            if let Some(requested_df) = get_df_promise.1.ready() {
                if let Ok(requested_df) = requested_df {
                    *self.required_dataframes.get_mut(&requested_df_id).unwrap() =
                        Some(requested_df.clone());
                }
                self.get_df_promise = None;
            }
        }

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
    }
}
