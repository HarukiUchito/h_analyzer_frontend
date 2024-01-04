use crate::components::modal_window::{self};
use polars::prelude::*;
use poll_promise::Promise;

pub use h_analyzer_data::grpc_data_transfer;
pub use h_analyzer_data::grpc_fs;

pub struct BackendTalk {
    server_address: String,
}

use tonic_web_wasm_client::Client;
impl BackendTalk {
    pub fn default() -> Self {
        BackendTalk {
            server_address: "http://192.168.64.2:50051".to_string(),
        }
    }

    pub fn get_world_list(
        &self,
    ) -> Promise<Result<grpc_data_transfer::WorldMetadataList, tonic::Status>> {
        let addr = self.server_address.clone();
        Promise::spawn_local(async move {
            let mut query_client =
                grpc_data_transfer::data_transfer2_d_client::DataTransfer2DClient::new(
                    Client::new(addr),
                );
            let req = grpc_data_transfer::Empty {};
            let resp = query_client.get_world_list(req).await?.into_inner();
            Ok(resp)
        })
    }

    pub fn request_default_path(&self) -> Promise<Result<grpc_fs::PathMessage, tonic::Status>> {
        let addr = self.server_address.clone();
        Promise::spawn_local(async move {
            let mut query_client =
                grpc_fs::file_system_client::FileSystemClient::new(Client::new(addr));
            let req = grpc_fs::Empty {};

            let resp = query_client.default_path(req).await?.into_inner();
            log::info!("resp: {:?}", resp);
            Ok(resp)
        })
    }

    pub fn request_list(
        &self,
        path: String,
    ) -> Promise<Result<grpc_fs::ListResponse, tonic::Status>> {
        let addr = self.server_address.clone();
        Promise::spawn_local(async move {
            let mut query_client =
                grpc_fs::file_system_client::FileSystemClient::new(Client::new(addr));
            let req = grpc_fs::PathMessage {
                path: path.to_string(),
            };

            let resp = query_client.list(req).await?.into_inner();
            Ok(resp)
        })
    }

    pub fn save_df_list(
        &self,
        df_info_list: Vec<grpc_fs::DataFrameInfo>,
    ) -> Promise<Result<grpc_fs::Empty, tonic::Status>> {
        let addr = self.server_address.clone();
        Promise::spawn_local(async move {
            let mut query_client =
                grpc_fs::polars_service_client::PolarsServiceClient::new(Client::new(addr));

            let req = grpc_fs::DataFrameInfoList { list: df_info_list };

            let resp = query_client.save_data_frame_list(req).await?.into_inner();
            Ok(resp)
        })
    }

    pub fn request_initial_df_list(
        &self,
    ) -> Promise<Result<grpc_fs::DataFrameInfoList, tonic::Status>> {
        let addr = self.server_address.clone();
        Promise::spawn_local(async move {
            let mut query_client =
                grpc_fs::polars_service_client::PolarsServiceClient::new(Client::new(addr));
            let req = grpc_fs::Empty {};

            let resp = query_client
                .initial_data_frame_list(req)
                .await?
                .into_inner();
            Ok(resp)
        })
    }

    pub fn load_df_request(
        &self,
        filepath: String,
        df_type: modal_window::DataFrameType,
    ) -> Promise<Result<DataFrame, tonic::Status>> {
        let base_url = self.server_address.clone();
        Promise::spawn_local(async move {
            let mut query_client = grpc_fs::polars_service_client::PolarsServiceClient::new(
                Client::new(base_url.to_string()),
            );

            let filetype = match df_type {
                modal_window::DataFrameType::CommaSep => grpc_fs::DataFrameType::CommaSep,
                modal_window::DataFrameType::NDEV => grpc_fs::DataFrameType::Ndev,
                modal_window::DataFrameType::KITTI => grpc_fs::DataFrameType::Kitti,
            };

            let req = grpc_fs::FileLoadRequest {
                filename: filepath,
                filetype: filetype.into(),
            };
            let mut stream = query_client.load_data_frame(req).await?.into_inner();

            let mut cvec = Vec::new();
            while let Some(cdata) = stream.message().await? {
                for v in cdata.data {
                    cvec.push(v);
                }
            }
            let df = bincode::deserialize_from(cvec.clone().as_slice()).unwrap();
            Ok(df)
        })
    }

    pub fn get_world_frame(
        &self,
        world_name: String,
        unix_timestamp: f64,
    ) -> Promise<Result<h_analyzer_data::WorldFrame, tonic::Status>> {
        let base_url = self.server_address.clone();
        Promise::spawn_local(async move {
            let mut query_client =
                grpc_data_transfer::data_transfer2_d_client::DataTransfer2DClient::new(
                    Client::new(base_url),
                );

            let req = grpc_data_transfer::GetWorldFrameRequest {
                id: Some(grpc_data_transfer::WorldId { id: world_name }),
                timestamp: Some(grpc_data_transfer::UnixTimeStamp {
                    value: unix_timestamp,
                }),
            };
            let mut stream = query_client.get_world_frame(req).await?.into_inner();

            let mut cvec = Vec::new();
            while let Some(cdata) = stream.message().await? {
                for v in cdata.data {
                    cvec.push(v);
                }
            }
            let wf = bincode::deserialize_from(cvec.clone().as_slice()).unwrap();
            Ok(wf)
        })
    }
}
