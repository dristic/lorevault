fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = "../../proto";

    tonic_prost_build::configure()
        .extern_path(".google.protobuf.Timestamp", "::prost_types::Timestamp")
        .compile_protos(
            &[
                "lore/model/v1/model.proto",
                "lore/environment/v1/environment.proto",
                "lore/repository/v1/repository.proto",
                "lore/revision/v1/revision.proto",
                "lore/storage/v1/storage.proto",
                "auth_api.proto",
                "lock.proto",
            ],
            &[proto_root],
        )?;

    Ok(())
}
