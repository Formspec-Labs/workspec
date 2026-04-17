//! Studio-facing view types. Each struct's JSON shape matches the
//! corresponding interface in `studio/src/services/{WosBackend,WosPorts}.ts`.

pub mod auth;

pub use auth::*;
