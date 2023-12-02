fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute(
            "grpc_fs.DataFrameType",
            "#[derive(serde::Deserialize, serde::Serialize)]",
        )
        .type_attribute(
            "grpc_fs.DataFrameInfo",
            "#[derive(serde::Deserialize, serde::Serialize)]",
        )
        .type_attribute(
            "grpc_fs.DataFrameInfoList",
            "#[derive(serde::Deserialize, serde::Serialize)]",
        )
        .compile(&["../protos/filesystem.proto"], &["../protos"])?;
    tonic_build::compile_protos("../protos/data_transfer.proto")?;
    Ok(())
}
