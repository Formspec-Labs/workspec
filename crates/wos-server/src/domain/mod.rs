//! Studio-facing view types. Each struct's JSON shape matches the
//! corresponding interface in `studio/src/services/{WosBackend,WosPorts}.ts`.

pub mod applicant;
pub mod auth;
pub mod bundle;
pub mod dashboard;
pub mod governance;
pub mod instance;
pub mod provenance;

pub use applicant::*;
pub use auth::*;
pub use bundle::*;
pub use dashboard::*;
pub use governance::*;
pub use instance::*;
pub use provenance::*;
