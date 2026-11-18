//! The HTML report for people (T-092; R-REPORT-1).
//!
//! [`model`] decides what the page shows from the parsed task lists; [`html`] writes it. Neither
//! changes a document, and the command that writes the file is in `cli::report`.

pub mod html;
pub mod model;

pub use html::render_page;
pub use model::{Page, build_page};
