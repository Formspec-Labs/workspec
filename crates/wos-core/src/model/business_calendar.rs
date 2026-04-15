// Rust guideline compliant 2026-04-14

//! Re-export of business calendar types from `crate::business_calendar`.
//!
//! The canonical definitions live in `crate::business_calendar::mod.rs`.
//! This module is kept for backward-compatibility with `model::business_calendar::*`
//! import paths used by downstream crates and tests.

pub use crate::business_calendar::{
    BusinessCalendarDocument, Holiday, OperatingHours, Weekday,
};
