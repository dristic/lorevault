//! Conversions between `lv-core`'s domain types and `lv-api-types`'s wire
//! types. Kept as plain functions (not `From` impls) since neither type is
//! local to this crate — Rust's orphan rule would block a trait impl here.

pub fn visibility_to_wire(v: lv_core::models::Visibility) -> lv_api_types::repos::Visibility {
    match v {
        lv_core::models::Visibility::Public => lv_api_types::repos::Visibility::Public,
        lv_core::models::Visibility::Private => lv_api_types::repos::Visibility::Private,
    }
}

pub fn repo_role_from_wire(r: lv_api_types::repos::RepoRole) -> lv_core::models::RepoRole {
    match r {
        lv_api_types::repos::RepoRole::Read => lv_core::models::RepoRole::Read,
        lv_api_types::repos::RepoRole::Write => lv_core::models::RepoRole::Write,
        lv_api_types::repos::RepoRole::Admin => lv_core::models::RepoRole::Admin,
    }
}
