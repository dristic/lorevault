// Hierarchically nested to match the package paths used by the generated
// cross-module references (e.g. `super::super::model::v1::Repository`).

pub mod lore {
    pub mod model {
        pub mod v1 {
            tonic::include_proto!("lore.model.v1");
        }
    }
    pub mod environment {
        pub mod v1 {
            tonic::include_proto!("lore.environment.v1");
        }
    }
    pub mod repository {
        pub mod v1 {
            tonic::include_proto!("lore.repository.v1");
        }
    }
    pub mod revision {
        pub mod v1 {
            tonic::include_proto!("lore.revision.v1");
        }
    }
    pub mod storage {
        pub mod v1 {
            tonic::include_proto!("lore.storage.v1");
        }
    }
}

pub mod epic_urc {
    tonic::include_proto!("epic_urc");
}

pub mod urc {
    pub mod lock {
        tonic::include_proto!("urc.lock");
    }
}

// Flat re-exports so service code can use `crate::proto::model::*` etc.
pub use lore::environment::v1 as environment;
pub use lore::model::v1 as model;
pub use lore::repository::v1 as repository;
pub use lore::revision::v1 as revision;
pub use lore::storage::v1 as storage;
pub use urc::lock;
pub use epic_urc as auth_api;
