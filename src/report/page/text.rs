//! Text from the task list as HTML (T-224; R-REPORT-1).

use crate::report::html::escape;

/// The text as HTML, every piece of it escaped (R-REPORT-1).
///
/// Text between a pair of backticks becomes a `code` element. Pairs are taken from the left, and
/// the text after each pair is read again for more pairs. A backtick without a partner, and a pair
/// with nothing between its backticks, stay as typed.
#[must_use]
pub fn inline_code(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('`') {
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('`') else {
            break;
        };
        let inside = &after_open[..close];
        if inside.is_empty() {
            out.push_str(&escape(&rest[..open + 2]));
        } else {
            out.push_str(&escape(&rest[..open]));
            out.push_str("<code>");
            out.push_str(&escape(inside));
            out.push_str("</code>");
        }
        rest = &after_open[close + 1..];
    }
    out.push_str(&escape(rest));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_only_escaped() {
        assert_eq!(
            inline_code("a <b> & \"c\" 'd'"),
            "a &lt;b&gt; &amp; &quot;c&quot; &#39;d&#39;"
        );
    }

    #[test]
    fn empty_text_stays_empty() {
        assert_eq!(inline_code(""), "");
    }

    #[test]
    fn a_pair_of_backticks_becomes_code() {
        assert_eq!(inline_code("a `b` c"), "a <code>b</code> c");
    }

    #[test]
    fn two_pairs_give_two_code_elements() {
        assert_eq!(
            inline_code("`todo add` and `todo done`"),
            "<code>todo add</code> and <code>todo done</code>"
        );
    }

    #[test]
    fn an_unpaired_backtick_stays_as_typed() {
        assert_eq!(inline_code("a `b c"), "a `b c");
    }

    #[test]
    fn an_unpaired_backtick_after_a_pair_stays_as_typed() {
        assert_eq!(inline_code("`a` b `"), "<code>a</code> b `");
    }

    #[test]
    fn two_backticks_with_nothing_between_stay_as_typed() {
        assert_eq!(inline_code("a `` b"), "a `` b");
        assert_eq!(inline_code("``"), "``");
    }

    #[test]
    fn a_pair_after_an_empty_pair_still_becomes_code() {
        assert_eq!(inline_code("`` `c`"), "`` <code>c</code>");
    }

    #[test]
    fn script_between_backticks_is_escaped_inside_the_code() {
        assert_eq!(
            inline_code("<b>`<script>`</b>"),
            "&lt;b&gt;<code>&lt;script&gt;</code>&lt;/b&gt;"
        );
    }

    #[test]
    fn text_outside_the_pairs_is_escaped_too() {
        assert_eq!(inline_code("a & `b` < c"), "a &amp; <code>b</code> &lt; c");
    }
}
