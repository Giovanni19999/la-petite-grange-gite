//! HTTP layer: axum routes, request/response DTOs and error mapping.
//!
//! Handlers stay thin: extract the request, call a use case, map to a DTO.
