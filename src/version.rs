//! The version of the running todo tool, and the `MAJOR.MINOR.PATCH` values written in the tool
//! stamp of `TODO.md` (`docs/format.md` §4, R-VER-1).

use std::fmt;

/// A `MAJOR.MINOR.PATCH` version of todo.
///
/// The fields compare in that order, so versions compare number by number: `0.10.0` is above
/// `0.9.0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToolVersion {
    /// Major number.
    pub major: u32,
    /// Minor number.
    pub minor: u32,
    /// Patch number.
    pub patch: u32,
}

/// The version of the running todo, taken from `Cargo.toml` at build time.
pub const RUNNING_VERSION: ToolVersion = ToolVersion {
    major: parse_component(env!("CARGO_PKG_VERSION_MAJOR")),
    minor: parse_component(env!("CARGO_PKG_VERSION_MINOR")),
    patch: parse_component(env!("CARGO_PKG_VERSION_PATCH")),
};

impl ToolVersion {
    /// Parses exactly three dot-separated decimal numbers, such as `0.6.0`.
    ///
    /// Returns `None` for anything else, including two or four numbers, empty parts, signs and
    /// letters. Surrounding spaces are not accepted here; callers trim the value first.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split('.');
        let major = parse_number(parts.next()?)?;
        let minor = parse_number(parts.next()?)?;
        let patch = parse_number(parts.next()?)?;
        if parts.next().is_some() {
            return None;
        }
        Some(Self {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for ToolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

fn parse_number(part: &str) -> Option<u32> {
    if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    part.parse().ok()
}

/// Reads the leading decimal digits of a Cargo version component at compile time.
const fn parse_component(text: &str) -> u32 {
    let bytes = text.as_bytes();
    let mut value: u32 = 0;
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        value = value * 10 + (bytes[i] - b'0') as u32;
        i += 1;
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(major: u32, minor: u32, patch: u32) -> ToolVersion {
        ToolVersion {
            major,
            minor,
            patch,
        }
    }

    #[test]
    fn parses_three_numbers_only() {
        assert_eq!(ToolVersion::parse("0.6.0"), Some(v(0, 6, 0)));
        assert_eq!(ToolVersion::parse("10.0.123"), Some(v(10, 0, 123)));
        for bad in [
            "",
            "0.6",
            "0.6.0.1",
            "0..1",
            "a.b.c",
            "+1.0.0",
            "-1.0.0",
            "1.0.0-rc1",
            "1. 0.0",
        ] {
            assert_eq!(ToolVersion::parse(bad), None, "{bad:?} must not parse");
        }
    }

    #[test]
    fn compares_number_by_number() {
        assert!(v(0, 10, 0) > v(0, 9, 0));
        assert!(v(0, 6, 0) > v(0, 5, 9));
        assert!(v(1, 0, 0) > v(0, 99, 99));
        assert!(v(0, 5, 0) == v(0, 5, 0));
    }

    #[test]
    fn displays_as_dotted_triple() {
        assert_eq!(v(0, 10, 2).to_string(), "0.10.2");
    }

    #[test]
    fn running_version_matches_cargo_version() {
        assert_eq!(
            RUNNING_VERSION.to_string(),
            env!("CARGO_PKG_VERSION"),
            "the compile-time parse must agree with Cargo.toml"
        );
    }
}
