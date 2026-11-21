//! The v1.0 page for one project (T-211; R-REPORT-1, R-REPORT-4 to R-REPORT-6; section 5.23).
//!
//! The page is built from the parts in this module. [`entry`] decides how one entry reads: its
//! state in words, its date, its second line and its details. The parts that place entries on the
//! page (the work tree, Needs you, Last 7 days) use it, so every entry reads the same everywhere.
//! [`super::model`] and [`super::html`] are not changed by this module.

pub mod entry;
pub mod needs;
