pub mod auth_api {
    tonic::include_proto!("epic_urc");
}

// The legacy urc.* packages are nested to match prost's cross-package super:: paths.
pub mod urc {
    pub mod model {
        tonic::include_proto!("urc.model");
    }
    pub mod rpc {
        tonic::include_proto!("urc.rpc");
    }
}

pub mod lore_environment_v1 {
    tonic::include_proto!("lore.environment.v1");
}
