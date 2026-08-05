//! Subscription matching for event-driven agent activation.
//!
//! Uses glob-style pattern matching against event types. Supports `*` (match all)
//! and `*.suffix` (suffix matching). Reused from descriptor.rs if available.

use crate::cognitive::AgentContext;
use crate::cognitive::ReactiveObserver;

// ---------------------------------------------------------------------------
// glob_match
// ---------------------------------------------------------------------------

/// Matches a glob pattern against a string.
///
/// Supports:
/// - `*` — matches everything
/// - `*.suffix` — matches strings ending with `.suffix`
/// - `exact` — exact string match
///
/// Returns `true` if `pattern` matches `text`.
pub(crate) fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        return text.ends_with(suffix);
    }
    pattern == text
}

// ---------------------------------------------------------------------------
// SubscriptionMatcher
// ---------------------------------------------------------------------------

/// Matches agent subscription patterns against event types.
///
/// An agent is activated when at least one of its subscription patterns
/// matches the dispatched event type.
pub struct SubscriptionMatcher;

impl SubscriptionMatcher {
    /// Returns `true` if `event_type` matches any pattern in `subscriptions`.
    ///
    /// Uses first-match: the first pattern that matches returns `true`.
    /// Empty subscriptions always return `false`.
    pub fn matches(subscriptions: &[String], event_type: &str) -> bool {
        if subscriptions.is_empty() {
            return false;
        }
        subscriptions.iter().any(|p| glob_match(p, event_type))
    }

    /// Returns the index of the first matching subscription, or `None`.
    pub fn match_index(subscriptions: &[String], event_type: &str) -> Option<usize> {
        subscriptions.iter().position(|p| glob_match(p, event_type))
    }
}

// ---------------------------------------------------------------------------
// Subscription-based filtering for ReactiveObserver
// ---------------------------------------------------------------------------

/// Extension trait that adds subscription-based filtering to ReactiveObserver.
pub trait SubscriptionFilter {
    /// Whether the agent has a matching subscription for the given event type.
    fn subscribed_to(&self, event_type: &str) -> bool;

    /// Whether the agent should be activated for the given event type and context.
    ///
    /// Combines subscription matching with the agent's own `matches()` check.
    fn should_activate(&self, event_type: &str, ctx: &AgentContext) -> bool;
}

impl<T: ReactiveObserver> SubscriptionFilter for T {
    fn subscribed_to(&self, event_type: &str) -> bool {
        SubscriptionMatcher::matches(&self.descriptor().subscriptions, event_type)
    }

    fn should_activate(&self, event_type: &str, ctx: &AgentContext) -> bool {
        self.subscribed_to(event_type) && self.matches(ctx)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_match_exact() {
        assert!(glob_match("GoalSubmitted", "GoalSubmitted"));
        assert!(!glob_match("GoalSubmitted", "GoalCancelled"));
    }

    #[test]
    fn glob_match_star() {
        assert!(glob_match("*", "anything.at.all"));
        assert!(glob_match("*", "GoalSubmitted"));
    }

    #[test]
    fn glob_match_suffix() {
        assert!(glob_match("*.changed", "file.changed"));
        assert!(glob_match("*.changed", "goalchanged"));
        assert!(!glob_match("*.changed", "GoalSubmitted"));
    }

    #[test]
    fn subscription_matcher_empty() {
        assert!(!SubscriptionMatcher::matches(&[], "GoalSubmitted"));
    }

    #[test]
    fn subscription_matcher_exact() {
        let subs = &["GoalSubmitted".into(), "FileChanged".into()];
        assert!(SubscriptionMatcher::matches(subs, "GoalSubmitted"));
        assert!(!SubscriptionMatcher::matches(subs, "GoalCancelled"));
    }

    #[test]
    fn subscription_matcher_star() {
        let subs = &["*".into()];
        assert!(SubscriptionMatcher::matches(subs, "anything"));
    }

    #[test]
    fn subscription_matcher_suffix() {
        let subs = &["*.changed".into()];
        assert!(SubscriptionMatcher::matches(subs, "file.changed"));
        assert!(!SubscriptionMatcher::matches(subs, "file.submitted"));
    }

    #[test]
    fn subscription_matcher_match_index() {
        let subs = &["GoalSubmitted".into(), "*.changed".into()];
        assert_eq!(
            SubscriptionMatcher::match_index(subs, "GoalSubmitted"),
            Some(0)
        );
        assert_eq!(
            SubscriptionMatcher::match_index(subs, "file.changed"),
            Some(1)
        );
        assert_eq!(
            SubscriptionMatcher::match_index(subs, "file.submitted"),
            None
        );
    }
}
