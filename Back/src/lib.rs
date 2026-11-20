//! REST API of the La Petite Grange holiday cottage website.
//!
//! Hexagonal architecture (see `docs/adr/0002-rust-axum-hexagonal-backend.md`):
//!
//! * [`domain`]: pure business rules, no I/O, no framework.
//! * [`application`]: use cases and ports (traits) implemented by adapters.
//! * [`infrastructure`]: adapters (`MariaDB`, providers…).
//! * [`http`]: axum routes, DTOs and error mapping only.

pub mod application;
pub mod domain;
pub mod http;
pub mod infrastructure;
