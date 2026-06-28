fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = "../../proto";

    tonic_prost_build::configure()
        .compile_protos(&["auth_api.proto"], &[proto_root])?;

    Ok(())
}
