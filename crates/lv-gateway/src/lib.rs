// tonic::Status is 176 bytes; Result<_, Status> is the idiomatic gRPC return type throughout this crate.
#![allow(clippy::result_large_err)]

pub mod jwt;
pub mod proto;
pub mod server;
pub mod services;
pub mod state;
