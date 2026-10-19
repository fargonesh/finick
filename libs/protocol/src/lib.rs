//! Finick-specific IPC protocol types.
//!
//! Transport lives in the generic `ipc` crate. This crate only defines
//! Finick domain messages (settings, notifications, clipboard, modals)
//! and thin helpers built on top of `ipc`.

#[doc(hidden)]
pub use crate as ipsea;

pub mod clipboard;
pub mod modals;
pub mod notifications;
pub mod settings;
