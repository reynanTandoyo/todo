//! The v1.0 page for one project (T-211, T-214; R-REPORT-1, section 5.23).
//!
//! [`build`] decides everything the page shows from the parsed task lists and the clock: the overall
//! counts, Needs you, Last 7 days, the work tree, and the closed parts of More. [`html::render`]
//! writes it. [`entry`] decides how one entry reads, and every part shows entries through it. The
//! planner's part (R-REPORT-3) is the planning state that [`super::model`] decides and
//! [`super::html`] writes. Read-only: nothing here changes a document.

pub mod chain;
pub mod entry;
pub mod html;
pub mod needs;
pub mod recent;
pub mod tree;
pub mod tree_html;

use chrono::{DateTime, FixedOffset, NaiveDate};

use super::model::{self, Planning};
use crate::doc::Document;
use crate::ops::OpsConfig;
use entry::Ctx;

/// Everything the page shows (R-REPORT-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageData {
    /// The project's title: the first `# ` heading of `TODO.md`, or "Tasks".
    pub title: String,
    /// The `description` setting (R-CFG-2), when it is set.
    pub description: Option<String>,
    /// The day of the build; the dates of the page are written relative to it.
    pub today: NaiveDate,
    /// The clock at the build, as the header shows it: "21 Nov 2026, 14:03".
    pub as_of: String,
    /// Tasks in each state, counted over `TODO.md` (the overall line).
    pub counts: tree::Counts,
    /// Needs you (R-REPORT-4), in order.
    pub needs: Vec<needs::Need>,
    /// Last 7 days (R-REPORT-5).
    pub recent: recent::Recent,
    /// All work (R-REPORT-1): the tree of `TODO.md`.
    pub tree: tree::Tree,
    /// Earlier work (More): the tree of `TODO-archive.md`; `None` when there is no archive.
    pub earlier: Option<tree::Earlier>,
    /// The titles of the open ideas, in file order (More, Ideas).
    pub ideas: Vec<String>,
    /// The planner's part (More, R-REPORT-3): the planning state and the counts per area, as
    /// [`super::model`] decides them.
    pub planner: model::Page,
}

/// Builds the page from the task list, its archive when there is one, the configuration and the
/// clock. `file_label` names `TODO.md` in the planner's part, as [`model::build_page`] takes it.
#[must_use]
pub fn build(
    doc: &Document,
    archive: Option<&Document>,
    cfg: &OpsConfig,
    now: DateTime<FixedOffset>,
    file_label: &str,
) -> PageData {
    let ctx = Ctx {
        doc,
        archive,
        cfg,
        now,
    };
    let tree = tree::build_tree(doc, &ctx);
    let mut planner = model::build_page(doc, archive, cfg, now, file_label);
    unlink_planner_rows(&mut planner.planning);
    PageData {
        title: planner.title.clone(),
        description: cfg
            .description
            .clone()
            .filter(|text| !text.trim().is_empty()),
        today: now.date_naive(),
        as_of: as_of_text(now),
        counts: tree.counts,
        needs: needs::build_needs(doc, &ctx),
        recent: recent::build_recent(doc, archive, &ctx),
        earlier: archive.map(|archive| tree::build_earlier(archive, &ctx)),
        ideas: planner
            .ideas
            .iter()
            .map(|idea| idea.title.clone())
            .collect(),
        tree,
        planner,
    }
}

/// The clock as the header shows it: "21 Nov 2026, 14:03".
fn as_of_text(now: DateTime<FixedOffset>) -> String {
    now.format("%-d %b %Y, %H:%M").to_string()
}

/// The rows of the planner's part do not link. The page writes no anchor for an entry, so a link
/// would lead nowhere; the rows name the task instead.
fn unlink_planner_rows(planning: &mut Planning) {
    let rows = planning
        .not_ready
        .iter_mut()
        .map(|item| &mut item.row)
        .chain(planning.given_back.iter_mut().map(|item| &mut item.row))
        .chain(
            planning
                .kept_on_purpose
                .iter_mut()
                .map(|item| &mut item.row),
        );
    for row in rows {
        row.linked = false;
    }
}
