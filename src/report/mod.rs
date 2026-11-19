//! The HTML reports for people (T-092, T-094; R-REPORT-1, R-REPORT-2).
//!
//! [`model`] decides what the single-project page shows from the parsed task lists, and [`all`]
//! what the page for every registered project shows; [`html`] writes both. None of them changes a
//! document, and the commands that write the files are in `cli::report`.

pub mod all;
pub mod html;
pub mod model;

pub use all::{AllPage, build_all_page};
pub use html::{render_all_page, render_page};
pub use model::{Page, build_page};
