#![recursion_limit = "1024"]
#![allow(
    clippy::items_after_test_module,
    clippy::too_many_arguments,
    clippy::type_complexity
)]

pub mod app;
pub mod auth;
pub mod config;
pub mod error;
pub mod pii;
pub mod repository;
pub mod routes;
