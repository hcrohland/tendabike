//! Re-export shim: all Strava driver items live in [`tb_provider::strava`].
//!
//! This crate exists so that existing consumers (`tb_exec`, `tb_axum`, `tb_sqlx`,
//! `tb_app`) keep compiling unchanged. New code should import from
//! [`tb_provider::strava`] directly.

pub use tb_provider::strava::*;
