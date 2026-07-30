//! Test entry point for `OxidOWL`
//!
//! This file serves as the main entry point for running all tests.

mod integration;
mod swrl;
mod unit;
mod roundtrip;

#[path = "helpers/mod.rs"]
pub mod helpers;
