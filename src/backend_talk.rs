use polars::prelude::*;
use poll_promise::Promise;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

pub mod grpc_fs {
    tonic::include_proto!("grpc_fs");
}

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

    pub fn load_df_request(&self, filepath: String) -> Promise<Result<DataFrame, tonic::Status>> {
        let base_url = self.server_address.clone();
        Promise::spawn_local(async move {
            let mut query_client = grpc_fs::polars_service_client::PolarsServiceClient::new(
                Client::new(base_url.to_string()),
            );
            let req = grpc_fs::FilenameRequest { filename: filepath };
            let mut stream = query_client.load_dataframe(req).await?.into_inner();

            let mut cvec = Vec::new();
            while let Some(cdata) = stream.message().await? {
                for v in cdata.data {
                    cvec.push(v);
                }
            }
            let df = bincode::deserialize_from(cvec.clone().as_slice()).unwrap();
            log::info!("{}", df);
            Ok(df)
        })
    }

    pub fn send_req_csv(&self) -> Promise<Result<(), tonic::Status>> {
        let base_url = self.server_address.clone();
        Promise::spawn_local(async move {
            let mut query_client = hello_world::operator_service_client::OperatorServiceClient::new(
                Client::new(base_url.to_string()),
            );
            let req = hello_world::OutputCsvOperatorRequest { group_id: 0 };
            let mut stream = query_client.output_csv(req).await?.into_inner();

            let filename = stream.message().await?;
            log::info!("filename: {:?}", filename);

            let mut cvec = Vec::new();
            while let Some(cdata) = stream.message().await? {
                match cdata.value {
                    Some(hello_world::operator_csv_file::Value::Data(dv)) => {
                        for v in dv {
                            cvec.push(v);
                        }
                    }
                    _ => (),
                }
            }
            let rdr = CsvReader::new(std::io::Cursor::new(&cvec));
            let df = rdr.finish().expect("csv reader error");

            log::info!("cvec: {:?}", cvec);
            log::info!("df: {:?}", df);

            Ok(())
        })
    }
}
