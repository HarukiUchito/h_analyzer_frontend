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
            server_address: "http://192.168.1.8:50051".to_string(),
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

    pub fn request_get_df_list(
        &self,
    ) -> Promise<Result<grpc_fs::DataFrameInfoList, tonic::Status>> {
        let addr = self.server_address.clone();
        Promise::spawn_local(async move {
            let mut query_client =
                grpc_fs::polars_service_client::PolarsServiceClient::new(Client::new(addr));
            let req = grpc_fs::Empty {};

            let resp = query_client.get_data_frame_list(req).await?.into_inner();
            Ok(resp)
        })
    }

    pub fn load_df_from_file_request(
        &self,
        filepath: String,
        load_option: h_analyzer_data::grpc_fs::DataFrameLoadOption,
    ) -> Promise<Result<usize, tonic::Status>> {
        let addr = self.server_address.clone();
        Promise::spawn_local(async move {
            let mut query_client =
                grpc_fs::polars_service_client::PolarsServiceClient::new(Client::new(addr));
            let req = grpc_fs::FileLoadRequest {
                filename: filepath,
                load_option: Some(load_option),
            };

            let resp = query_client
                .load_data_frame_from_file(req)
                .await?
                .into_inner();
            Ok(resp.id as usize)
        })
    }

    pub fn load_df_request(
        &self,
        filepath: String,
        load_option: h_analyzer_data::grpc_fs::DataFrameLoadOption,
    ) -> Promise<Result<DataFrame, tonic::Status>> {
        let base_url = self.server_address.clone();
        Promise::spawn_local(async move {
            let mut query_client = grpc_fs::polars_service_client::PolarsServiceClient::new(
                Client::new(base_url.to_string()),
            );

            let req = grpc_fs::FileLoadRequest {
                filename: filepath,
                load_option: Some(load_option),
            };
            let mut stream = query_client.load_data_frame(req).await?.into_inner();

            let mut cvec = Vec::new();
            while let Some(cdata) = stream.message().await? {
                for v in cdata.data {
                    cvec.push(v);
                }
            }
            let df = bincode::deserialize_from(cvec.clone().as_slice()).unwrap_or_default();
            Ok(df)
        })
    }

    pub fn remove_df_request(
        &self,
        id: h_analyzer_data::grpc_fs::DataFrameId,
    ) -> Promise<Result<h_analyzer_data::grpc_fs::Empty, tonic::Status>> {
        let base_url = self.server_address.clone();
        Promise::spawn_local(async move {
            let mut query_client = grpc_fs::polars_service_client::PolarsServiceClient::new(
                Client::new(base_url.to_string()),
            );

            Ok(query_client.remove_data_frame(id).await?.into_inner())
        })
    }

    pub fn get_df_request(
        &self,
        id: h_analyzer_data::grpc_fs::DataFrameId,
    ) -> Promise<Result<DataFrame, tonic::Status>> {
        let base_url = self.server_address.clone();
        Promise::spawn_local(async move {
            let mut query_client = grpc_fs::polars_service_client::PolarsServiceClient::new(
                Client::new(base_url.to_string()),
            );

            let mut stream = query_client.get_data_frame(id).await?.into_inner();

            let mut cvec = Vec::new();
            while let Some(cdata) = stream.message().await? {
                for v in cdata.data {
                    cvec.push(v);
                }
            }
            let df = bincode::deserialize_from(cvec.clone().as_slice()).unwrap_or_default();
            Ok(df)
        })
    }

    pub fn get_world_frame(
        &self,
        world_name: String,
        frame_index: u32,
    ) -> Promise<Result<h_analyzer_data::WorldFrame, tonic::Status>> {
        let base_url = self.server_address.clone();
        Promise::spawn_local(async move {
            let mut query_client =
                grpc_data_transfer::data_transfer2_d_client::DataTransfer2DClient::new(
                    Client::new(base_url),
                );

            let req = grpc_data_transfer::GetWorldFrameRequest {
                id: Some(grpc_data_transfer::WorldId { id: world_name }),
                request_type: grpc_data_transfer::WorldFrameRequestType::FrameIndex.into(),
                frame_index: frame_index,
                timestamp: Some(grpc_data_transfer::UnixTimeStamp { value: 0.0 }),
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
