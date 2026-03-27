//! Core library for `illfit`.
//!
//! The project starts with data ingestion because the numerical pipeline will be
//! much easier to develop once we have a validated SAXS curve type that all
//! later modules can rely on.

#![forbid(unsafe_code)]

pub mod data;
