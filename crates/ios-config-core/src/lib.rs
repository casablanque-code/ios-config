#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

pub mod ir;
pub use ir::*;

// The IR types are documented in the `ir` module. Struct fields that are
// self-evident (e.g. `asn: u32`, `name: String`) carry `#[allow(missing_docs)]`
// at the module level in ir.rs rather than cluttering every field.
