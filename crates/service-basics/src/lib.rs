// SPDX-License-Identifier: MIT

//! What every Rust port of a gluon service needs at runtime (gluon TD-43):
//! the pieces user-service-rs and notification-service-rs both had a copy of.

pub mod config;
pub mod health;
pub mod postgres;
pub mod text_error;
