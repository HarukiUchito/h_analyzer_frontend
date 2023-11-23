use crate::backend_talk;
use crate::components::modal_window;
use polars::prelude::*;
use poll_promise::Promise;
use std::collections::HashMap;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct CommonData {
    pub modal_window_open: bool,
    pub dataframe_info: Option<modal_window::DataFrameInfo>,

    pub dataframes: HashMap<String, DataFrame>,
    pub current_path: String,
    pub default_path: String,

    #[serde(skip)]
    pub backend: backend_talk::BackendTalk,
    #[serde(skip)]
    pub fs_list_promise:
        Option<Promise<Result<backend_talk::grpc_fs::ListResponse, tonic::Status>>>,
    #[serde(skip)]
    pub hello_promise: Option<(String, Promise<Result<DataFrame, tonic::Status>>)>,
    #[serde(skip)]
    pub d_path_promise: Option<Promise<Result<backend_talk::grpc_fs::PathMessage, tonic::Status>>>,
}

impl Default for CommonData {
    fn default() -> Self {
        let path = "/".to_string();
        let backend = backend_talk::BackendTalk::default();
        let fs_list_promise = backend.request_list(path.clone());
        let d_path_promise = backend.request_default_path();
        Self {
            modal_window_open: false,
            dataframe_info: None,

            backend: backend,
            dataframes: HashMap::new(),
            current_path: path.clone(),
            default_path: path.clone(),
            fs_list_promise: Some(fs_list_promise),
            hello_promise: None,
            d_path_promise: Some(d_path_promise),
        }
    }
}

impl CommonData {
    pub fn update(&mut self) {
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
            if let Some((dname, result)) = &self.hello_promise {
                if let Some(result) = result.ready() {
                    if let Ok(result) = result {
                        self.dataframes.insert(dname.clone(), result.clone());
                        return result.clone();
                    }
                }
            }
            DataFrame::default()
        })();

        if let Some(df_info) = self.dataframe_info.as_mut() {
            if df_info.load_now {
                let name = modal_window::get_filename(df_info.filepath.as_str());
                self.hello_promise =
                    Some((name, self.backend.load_df_request(df_info.filepath.clone())));
                self.modal_window_open = false;
                df_info.load_now = false;
            }
        }
    }
}
