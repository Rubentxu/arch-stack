//! M73 T3: Channel resolution for self-update (stable / rc / nightly).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Stable,
    Rc,
    Nightly,
}

impl std::str::FromStr for Channel {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stable" => Ok(Self::Stable),
            "rc" => Ok(Self::Rc),
            "nightly" => Ok(Self::Nightly),
            _ => Err(format!("unknown channel: {s}")),
        }
    }
}

/// Format channel prefix for GitHub tag lookup.
/// - Stable: "" (latest stable release, no prefix)
/// - Rc: "" (latest stable or rc; caller filters by tag name)
/// - Nightly: "nightly-" prefix (e.g. "nightly-2026-08-10")
pub fn channel_label(chan: Channel) -> &'static str {
    match chan {
        Channel::Stable | Channel::Rc => "",
        Channel::Nightly => "nightly-",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_parses_from_str() {
        assert_eq!("stable".parse::<Channel>().unwrap(), Channel::Stable);
        assert_eq!("rc".parse::<Channel>().unwrap(), Channel::Rc);
        assert_eq!("nightly".parse::<Channel>().unwrap(), Channel::Nightly);
        assert!("foo".parse::<Channel>().is_err());
    }

    #[test]
    fn channel_label_for_stable_is_empty() {
        assert_eq!(channel_label(Channel::Stable), "");
    }

    #[test]
    fn channel_label_for_nightly_has_prefix() {
        assert_eq!(channel_label(Channel::Nightly), "nightly-");
    }

    #[test]
    fn channel_label_for_rc_is_empty() {
        assert_eq!(channel_label(Channel::Rc), "");
    }
}
