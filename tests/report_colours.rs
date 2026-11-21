//! Colour tests of the v1.0 page (T-203; R-REPORT-6): the four colour sets in the stylesheet hold
//! exactly the colours of the requirement's table, and each state colour has a contrast of at least
//! 3 to 1 against the background of its mode (WCAG relative luminance). The stylesheet is read from
//! the source tree, so the test checks the colours the page is written with. Every value is invented
//! for the test or taken from the requirement.

/// The stylesheet the page copies in (`src/report/page/style.css`).
const STYLE: &str = include_str!("../src/report/page/style.css");

/// The state colours in the order of the table: done, in progress, waiting, to do, cancelled.
const STATES: [&str; 5] = ["done", "prog", "wait", "todo", "drop"];

/// One colour set of R-REPORT-6.
struct Set {
    /// The name in the messages.
    name: &'static str,
    /// The selectors whose blocks hold the set. The dark set appears twice: under the system setting
    /// (no mode chosen) and when dark mode is chosen.
    selectors: &'static [&'static str],
    /// The page background of the set.
    background: &'static str,
    /// The state colours in the order of `STATES`, as the requirement's table gives them.
    colours: [&'static str; 5],
}

const SETS: [Set; 4] = [
    Set {
        name: "light",
        selectors: &[".page{"],
        background: "#ffffff",
        colours: ["#2f9e5f", "#3b7fe0", "#b07800", "#80868e", "#cf4a4a"],
    },
    Set {
        name: "dark",
        selectors: &[".page:not([data-theme]){", ".page[data-theme=\"dark\"]{"],
        background: "#17181b",
        colours: ["#4cc17f", "#6aa5f5", "#e0a526", "#8b9098", "#ee6b6b"],
    },
    Set {
        name: "light, colour-blind",
        selectors: &[".page[data-cb=\"on\"]{"],
        background: "#ffffff",
        colours: ["#0072b2", "#a86400", "#a86400", "#8e949c", "#33363b"],
    },
    Set {
        name: "dark, colour-blind",
        selectors: &[
            ".page:not([data-theme])[data-cb=\"on\"]{",
            ".page[data-theme=\"dark\"][data-cb=\"on\"]{",
        ],
        background: "#17181b",
        colours: ["#56b4e9", "#f0a830", "#f0a830", "#7d838c", "#dcdfe3"],
    },
];

/// The text of the block that starts with `selector` (after its `{`, up to its `}`).
fn block<'a>(css: &'a str, selector: &str) -> Option<&'a str> {
    let start = css.find(selector)? + selector.len();
    let rest = &css[start..];
    Some(&rest[..rest.find('}')?])
}

/// The value that `block` declares for the custom property or property `name`, without its `;`.
fn declared<'a>(block: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}:");
    let start = block.find(&key)? + key.len();
    let rest = &block[start..];
    Some(rest[..rest.find(';').unwrap_or(rest.len())].trim())
}

/// The red, green and blue values of a `#rrggbb` colour.
fn rgb(hex: &str) -> Option<[u8; 3]> {
    let digits = hex.strip_prefix('#')?;
    if digits.len() != 6 {
        return None;
    }
    let part = |at: usize| u8::from_str_radix(digits.get(at..at + 2)?, 16).ok();
    Some([part(0)?, part(2)?, part(4)?])
}

/// The WCAG relative luminance of an sRGB colour, from 0 (black) to 1 (white).
fn luminance(rgb: [u8; 3]) -> f64 {
    let linear = |channel: u8| {
        let c = f64::from(channel) / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(rgb[0]) + 0.7152 * linear(rgb[1]) + 0.0722 * linear(rgb[2])
}

/// The WCAG contrast ratio of two colours, from 1 (no contrast) to 21 (black on white).
fn contrast(a: [u8; 3], b: [u8; 3]) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (lighter, darker) = if la > lb { (la, lb) } else { (lb, la) };
    (lighter + 0.05) / (darker + 0.05)
}

/// The five state colours that the block of `selector` declares, in the order of `STATES`.
fn state_colours(selector: &str) -> Option<[&'static str; 5]> {
    let body = block(STYLE, selector)?;
    let mut colours = [""; 5];
    for (slot, state) in colours.iter_mut().zip(STATES) {
        *slot = declared(body, &format!("--c-{state}"))?;
    }
    Some(colours)
}

#[test]
fn each_colour_set_declares_the_state_colours_of_the_table_in_every_block_that_holds_it() {
    for set in &SETS {
        for selector in set.selectors {
            let found = state_colours(selector);
            assert_eq!(
                found,
                Some(set.colours),
                "the {} set in {selector} does not hold the table's colours",
                set.name
            );
        }
    }
}

#[test]
fn each_state_colour_has_a_contrast_of_at_least_3_to_1_in_all_four_sets() {
    for set in &SETS {
        let background = rgb(set.background).expect("a background colour of the table");
        for selector in set.selectors {
            let colours = state_colours(selector).expect("a colour set in the stylesheet");
            for (state, colour) in STATES.iter().zip(colours) {
                let value = rgb(colour).expect("a state colour in #rrggbb form");
                let ratio = contrast(value, background);
                assert!(
                    ratio >= 3.0,
                    "{} set, {state}: {colour} on {} has a contrast of {ratio:.2} to 1",
                    set.name,
                    set.background
                );
            }
        }
    }
}

#[test]
fn the_dark_sets_apply_to_the_system_setting_until_a_mode_is_chosen() {
    let media = STYLE
        .find("@media (prefers-color-scheme: dark){")
        .expect("a dark rule for the system setting");
    let system_set = STYLE
        .find(".page:not([data-theme]){")
        .expect("the dark set for the system setting");
    assert!(
        system_set > media,
        "the system dark set is not under the media rule"
    );
    let body = block(STYLE, ".page:not([data-theme]){").expect("the block");
    assert!(declared(body, "color-scheme") == Some("dark"));
}

#[test]
fn the_colour_blind_bar_parts_and_the_waiting_stripes_are_the_ones_of_the_table() {
    // Bar part of in progress, bar part of to do, and the waiting stripes: the orange, then the
    // stripe colour of the set's background.
    for (selector, progress, to_do, stripe) in [
        (".page[data-cb=\"on\"]{", "#e69f00", "#c9ccd1", "#1f1f1f"),
        (
            ".page:not([data-theme])[data-cb=\"on\"]{",
            "#f0a830",
            "#4a4e55",
            "#17181b",
        ),
        (
            ".page[data-theme=\"dark\"][data-cb=\"on\"]{",
            "#f0a830",
            "#4a4e55",
            "#17181b",
        ),
    ] {
        let body = block(STYLE, selector).expect("a colour-blind block");
        assert_eq!(declared(body, "--s-prog"), Some(progress), "{selector}");
        assert_eq!(declared(body, "--s-todo"), Some(to_do), "{selector}");
        let waiting = declared(body, "--s-wait").expect("a waiting bar part");
        let pattern = format!("{progress} 0 4px,{stripe} 4px 8px");
        assert!(waiting.contains(&pattern), "{selector}: {waiting}");
    }
}

#[test]
fn the_date_of_a_done_or_cancelled_entry_uses_the_muted_text_colour_in_every_mode() {
    let rule = block(STYLE, ".when,.id{").expect("the rule of the date and the ID");
    assert_eq!(declared(rule, "color"), Some("var(--p-mfg)"));
    for selector in [
        ".page{",
        ".page:not([data-theme]){",
        ".page[data-theme=\"light\"]{",
        ".page[data-theme=\"dark\"]{",
    ] {
        let body = block(STYLE, selector).expect("a mode block");
        assert!(
            declared(body, "--p-mfg").is_some(),
            "{selector} does not set the muted text colour"
        );
    }
}
