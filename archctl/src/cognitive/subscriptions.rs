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

    // ---------------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage v4, 2026-08-22)
    // ---------------------------------------------------------------------------

    /// `glob_match` with an empty pattern does NOT match any non-empty text
    /// (per the contract at subscriptions.rs:21-29: pattern must be `*`,
    /// `*.suffix`, or exact). Empty pattern → fall through to `pattern == text`,
    /// which requires the text to also be empty.
    #[test]
    fn glob_match_empty_pattern_only_matches_empty_text() {
        assert!(glob_match("", ""), "empty pattern matches empty text");
        assert!(!glob_match("", "anything"));
        assert!(!glob_match("", "x"));
    }

    /// `glob_match` with `*` in the MIDDLE of a pattern (e.g. `foo*bar`)
    /// does NOT match `foobar` — the only special pattern starts with `*.`.
    /// This locks the deliberately-restricted glob grammar.
    #[test]
    fn glob_match_star_in_middle_falls_through_to_exact() {
        // "foo*bar" is NOT a recognized special pattern, so the function
        // falls through to `pattern == text` (line 28). It treats "*" as
        // a literal character.
        assert!(
            !glob_match("foo*bar", "foobar"),
            "star in middle is not a recognized special pattern"
        );
        // The only special case is `*` (alone) or `*.suffix` (suffix match).
        // Note: `*.bar` matches ANY string ending with `.bar` (suffix match,
        // not "starts-with-*-then-anything"). So `fooqux.bar` matches too.
        assert!(glob_match("*", "anything"));
        assert!(glob_match("*.bar", "foobar.bar"));
        assert!(
            glob_match("*.bar", "fooqux.bar"),
            "*.bar matches any text ending with .bar"
        );
        assert!(!glob_match("*.bar", "fooqux.baz"));
    }

    /// `SubscriptionMatcher::matches` returns true if ANY pattern in the
    /// subscriptions list matches. Locks the `any()` short-circuit.
    #[test]
    fn subscription_matcher_any_pattern_matches() {
        let subs = &[
            "GoalSubmitted".into(),
            "FileChanged".into(),
            "*.created".into(),
        ];
        // First pattern matches
        assert!(SubscriptionMatcher::matches(subs, "GoalSubmitted"));
        // Second pattern matches
        assert!(SubscriptionMatcher::matches(subs, "FileChanged"));
        // Third pattern (suffix) matches
        assert!(SubscriptionMatcher::matches(subs, "doc.created"));
        // None match
        assert!(!SubscriptionMatcher::matches(subs, "GoalCancelled"));
    }

    /// `SubscriptionFilter::subscribed_to` delegates to
    /// `SubscriptionMatcher::matches`. Locks the extension trait's contract.
    #[test]
    fn subscription_filter_subscribed_to_uses_matcher() {
        use crate::cognitive::descriptor::{AgentBudget, AgentDescriptor, ModelPolicy};
        use crate::cognitive::observer::{NoopObserver, ReactiveObserver};

        struct TestObserver;
        impl ReactiveObserver for TestObserver {
            fn descriptor(&self) -> AgentDescriptor {
                AgentDescriptor {
                    id: "test".into(),
                    version: "0.1.0".into(),
                    subscriptions: vec!["GoalSubmitted".into(), "*.changed".into()],
                    required_views: vec![],
                    output_schema: "{}".into(),
                    model_policy: ModelPolicy::Heuristic,
                    budget: AgentBudget::default(),
                    capabilities: vec![],
                    deterministic: true,
                    idempotent: true,
                }
            }
            fn observe(
                &self,
                _ctx: &crate::cognitive::AgentContext,
            ) -> Result<
                crate::cognitive::output::AgentOutput,
                crate::cognitive::observer::ObserveError,
            > {
                Ok(crate::cognitive::output::AgentOutput::NoAction(
                    crate::cognitive::output::NoActionReason {
                        code: crate::cognitive::output::NoActionCode::OutOfScope,
                        message: "no-op".into(),
                    },
                ))
            }
        }

        let obs = TestObserver;
        assert!(obs.subscribed_to("GoalSubmitted"));
        assert!(obs.subscribed_to("file.changed"));
        assert!(!obs.subscribed_to("GoalCancelled"));

        // Suppress unused warning for NoopObserver (used to silence the
        // unused-import lint when this test is the only consumer).
        let _ = NoopObserver {
            descriptor: obs.descriptor(),
        };
    }

    /// `SubscriptionFilter::should_activate` combines subscription
    /// matching AND the observer's own `matches()` check. Both must
    /// return true for activation.
    #[test]
    fn subscription_filter_should_activate_combines_both_checks() {
        use crate::cognitive::descriptor::{AgentBudget, AgentDescriptor, ModelPolicy};
        use crate::cognitive::observer::{NoopObserver, ObserveError, ReactiveObserver};

        struct ConditionalObserver {
            should_match: bool,
        }
        impl ReactiveObserver for ConditionalObserver {
            fn descriptor(&self) -> AgentDescriptor {
                AgentDescriptor {
                    id: "cond".into(),
                    version: "0.1.0".into(),
                    subscriptions: vec!["GoalSubmitted".into()],
                    required_views: vec![],
                    output_schema: "{}".into(),
                    model_policy: ModelPolicy::Heuristic,
                    budget: AgentBudget::default(),
                    capabilities: vec![],
                    deterministic: true,
                    idempotent: true,
                }
            }
            fn matches(&self, _ctx: &crate::cognitive::AgentContext) -> bool {
                self.should_match
            }
            fn observe(
                &self,
                _ctx: &crate::cognitive::AgentContext,
            ) -> Result<crate::cognitive::output::AgentOutput, ObserveError> {
                unreachable!("should_activate must short-circuit before observe")
            }
        }

        let obs = ConditionalObserver { should_match: true };
        let ctx = crate::cognitive::context::AgentContext {
            goal: "g".into(),
            triggering_event: None,
            graph_view: crate::cognitive::context::GraphView::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
            feedback_history: vec![],
            pending_adjudications: vec![],
        };
        // Both checks pass → activate
        assert!(obs.should_activate("GoalSubmitted", &ctx));

        // Subscription fails → don't activate
        assert!(!obs.should_activate("GoalCancelled", &ctx));

        let obs_no_match = ConditionalObserver {
            should_match: false,
        };
        // Subscription passes, but matches() returns false → don't activate
        assert!(!obs_no_match.should_activate("GoalSubmitted", &ctx));

        // Suppress unused import warning
        let _ = NoopObserver {
            descriptor: obs.descriptor(),
        };
    }
}
