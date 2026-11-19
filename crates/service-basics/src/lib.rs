// SPDX-License-Identifier: MIT

//! What every Rust port of a gluon service needs at runtime (gluon TD-43):
//! the pieces user-service-rs and notification-service-rs both had a copy of.

pub mod config;
pub mod cors;
pub mod edge;
pub mod environment;
pub mod health;
pub mod jvm;
pub mod postgres;
pub mod runtime;
pub mod shutdown;
pub mod text_error;
