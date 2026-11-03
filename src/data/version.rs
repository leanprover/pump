use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

/// Parse one number of a version.
fn parse_number(s: &str) -> Option<u64> {
    // No signs etc.
    if !s.bytes().all(|it| it.is_ascii_digit()) {
        return None;
    }

    // No leading zeroes allowed
    if s.starts_with('0') && s != "0" {
        return None;
    }

    s.parse::<u64>().ok()
}

/// Parse exactly `N` dot-separated decimal numbers.
fn parse_parts<const N: usize>(s: &str) -> Option<[u64; N]> {
    s.split('.')
        .map(parse_number)
        .collect::<Option<Vec<_>>>()?
        .try_into()
        .ok()
}

/// Impeller's application version, `major.minor.patch`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AppVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl FromStr for AppVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let [major, minor, patch] =
            parse_parts(s).ok_or_else(|| format!("invalid app version: {s:?}"))?;
        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for AppVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl fmt::Debug for AppVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl TryFrom<String> for AppVersion {
    type Error = <Self as FromStr>::Err;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<AppVersion> for String {
    fn from(value: AppVersion) -> Self {
        value.to_string()
    }
}

/// The output version of an impeller command, `major.minor`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct OutputVersion {
    pub major: u64,
    pub minor: u64,
}

impl FromStr for OutputVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let [major, minor] =
            parse_parts(s).ok_or_else(|| format!("invalid output version: {s:?}"))?;
        Ok(Self { major, minor })
    }
}

impl fmt::Display for OutputVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl fmt::Debug for OutputVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl TryFrom<String> for OutputVersion {
    type Error = <Self as FromStr>::Err;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<OutputVersion> for String {
    fn from(value: OutputVersion) -> Self {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(s: &str) -> AppVersion {
        s.parse().unwrap()
    }

    fn out(s: &str) -> OutputVersion {
        s.parse().unwrap()
    }

    #[test]
    fn parse() {
        assert_eq!(
            app("1.22.333"),
            AppVersion {
                major: 1,
                minor: 22,
                patch: 333
            }
        );

        assert_eq!(
            out("1.22"),
            OutputVersion {
                major: 1,
                minor: 22
            }
        );
    }

    #[test]
    fn reject() {
        for bad in [
            "",
            "1",
            "1.2",
            "1.2.3.4",
            ".2.3",
            "1..3",
            "1.2.",
            "+1.2.3",
            "-1.2.3",
            "01.2.3",
            "1,2,3",
            "1.2.x",
            " 1.2.3",
            "1. 2.3",
            "1.2.3 ",
            "1.2.3-rc1",
            "1.2.3+build",
            "1_0.2.3",
            "1.2.3.",
            ".1.2.3",
            "v1.2.3",
            "１.2.3",
        ] {
            assert!(bad.parse::<AppVersion>().is_err(), "{bad:?}");
        }

        for bad in [
            "", "1", "1.2.3", ".1", "1.", "-1.2", "+1.2", "01.2", "1,2", "1.x", "1..2", " 1.2",
            "1. 2", "1.2 ", "1_0.2", "1.2.", ".1.2", "v1.2", "１.2",
        ] {
            assert!(bad.parse::<OutputVersion>().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn roundtrip() {
        for s in ["0.0.0", "0.0.1", "0.1.0", "1.0.0", "1.22.333"] {
            assert_eq!(app(s).to_string(), s);
            assert_eq!(
                serde_json::from_str::<AppVersion>(&serde_json::to_string(&app(s)).unwrap())
                    .unwrap(),
                app(s)
            );
        }

        for s in ["0.0", "0.1", "1.0", "1.22"] {
            assert_eq!(out(s).to_string(), s);
            assert_eq!(
                serde_json::from_str::<OutputVersion>(&serde_json::to_string(&out(s)).unwrap())
                    .unwrap(),
                out(s)
            );
        }
    }

    #[test]
    fn ordering() {
        assert!(app("0.0.0") < app("0.0.1"));
        assert!(app("0.0.0") < app("0.1.0"));
        assert!(app("0.0.0") < app("1.0.0"));
        assert!(app("0.0.1") < app("0.1.0"));
        assert!(app("0.0.1") < app("1.0.0"));
        assert!(app("0.1.0") < app("1.0.0"));

        assert!(app("1.2.3") < app("1.2.5"));
        assert!(app("1.2.3") < app("1.3.0"));
        assert!(app("1.2.3") < app("3.2.1"));

        assert!(out("0.0") < out("0.1"));
        assert!(out("0.0") < out("1.0"));
        assert!(out("0.1") < out("1.0"));

        assert!(out("1.2") < out("1.4"));
        assert!(out("1.2") < out("4.0"));
    }

    #[test]
    fn serde() {
        assert_eq!(serde_json::to_string(&app("1.2.3")).unwrap(), "\"1.2.3\"");
        assert_eq!(serde_json::to_string(&out("1.2")).unwrap(), "\"1.2\"");
    }
}
