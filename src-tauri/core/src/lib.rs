//! Everything GuitarStar does to a Rocksmith install, kept free of UI code so
//! it can be tested on its own and reused by other front ends.

pub mod audio;
pub mod cdlc;
pub mod components;
pub mod display;
pub mod error;
pub mod game;
pub mod i18n;
pub mod ini;
pub mod linux;
pub mod rsmods;
pub mod saves;
pub mod snapshot;
pub mod sources;
pub mod state;
pub mod steam;
pub mod versions;

pub use reqwest;
