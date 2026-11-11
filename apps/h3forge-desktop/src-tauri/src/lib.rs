//! H3Forge desktop application library.
//!
//! Owns the session state for the single open save, the serde DTOs exchanged
//! with the frontend, and the Tauri command surface. All parsing stays in the
//! `h3forge-save` core crate; this layer only grants, reads, and projects.

pub mod commands;
pub mod dialog_dir;
pub mod dto;
pub mod installation;
pub mod localization;
pub mod map_link;
pub mod session;

pub mod neutral;
pub mod player_details;
