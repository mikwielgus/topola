#![doc(
    html_favicon_url = "https://codeberg.org/topola/topola/raw/commit/e1b56875edf039aab9f41868826bcd3a92097133/assets/favicon.ico"
)]
#![doc(
    html_logo_url = "https://codeberg.org/topola/topola/raw/commit/e1b56875edf039aab9f41868826bcd3a92097133/assets/logo.svg"
)]
#![cfg_attr(not(feature = "disable_contracts"), feature(try_blocks))]

pub mod graph;
#[macro_use]
pub mod drawing;
pub mod autorouter;
pub mod board;
pub mod geometry;
pub mod interactor;
pub mod layout;
pub mod math;
pub mod router;
pub mod specctra;
pub mod stepper;
pub mod triangulation;
