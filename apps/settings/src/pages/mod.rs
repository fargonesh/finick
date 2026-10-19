#![allow(dead_code)]
#![allow(unused_imports)]

pub mod network;
pub mod mouse;
pub mod bluetooth;
pub mod storage;
pub mod about;
pub mod date_time;
pub mod privacy;
pub mod language;
pub mod printers;
pub mod shortcuts;
pub mod default_apps;
pub mod online_accounts;
pub mod sharing;
pub mod search;
pub mod apps;
pub mod keyboard;
pub mod multitasking;
pub mod system;
pub mod peripherals;

pub use network::Network;
pub use mouse::Mouse;
pub use bluetooth::Bluetooth;
pub use storage::Storage;
pub use about::About;
pub use date_time::DateTime;
pub use privacy::Privacy;
pub use language::Language;
pub use printers::Printers;
pub use shortcuts::Shortcuts;
pub use default_apps::DefaultApps;
pub use online_accounts::OnlineAccounts;
pub use sharing::Sharing;
pub use search::Search;
pub use apps::Apps;
pub use keyboard::Keyboard;
pub use multitasking::Multitasking;
pub use system::System;
pub use peripherals::Peripherals;
