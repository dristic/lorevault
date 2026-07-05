fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = "../../proto";

    // Our protos use proto3 `optional` fields, which some CI runners' bundled
    // protoc (pre-3.15) still gates behind this flag rather than treating as stable.
    tonic_prost_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(
            &[
                "auth_api.proto",
                "environment.proto",
                "lore/environment/v1/environment.proto",
                "rebac_api.proto",
            ],
            &[proto_root],
        )?;

    Ok(())
}
