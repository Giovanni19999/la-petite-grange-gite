//! Pure business rules.
//!
//! Nothing in this module may depend on axum, sqlx, reqwest or any I/O.
//! Business values (delays, percentages, amounts…) are never hard-coded here:
//! they are received as settings (see `docs/adr/0010-no-business-value-hardcoded.md`).

pub mod auth;
pub mod media;

pub mod page;
