//! Deterministic [`wos_synth_core::Prompter`] for tests and CI smoke runs.
//!
//! `MockPrompter` matches incoming prompts by substring against pre-registered
//! responses. The first registered fragment that the user prompt contains
//! determines the canned reply. Unregistered prompts fail loudly so test
//! authors notice when the loop drifts off the rails they expected.

use std::sync::Mutex;

use async_trait::async_trait;
use wos_synth_core::{CacheAnchor, Completion, Prompter, PrompterError};

/// One canned response keyed by a substring the user prompt must contain.
struct Expectation {
    fragment: String,
    response: String,
    /// Cap on how many times this expectation may be served. `None` = unbounded.
    remaining: Option<u32>,
}

/// Deterministic prompter for tests.
#[derive(Default)]
pub struct MockPrompter {
    expectations: Mutex<Vec<Expectation>>,
}

impl MockPrompter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a canned response whenever the user prompt contains `fragment`.
    pub fn expect(&self, fragment: impl Into<String>, response: impl Into<String>) -> &Self {
        self.expectations.lock().unwrap().push(Expectation {
            fragment: fragment.into(),
            response: response.into(),
            remaining: None,
        });
        self
    }

    /// Register a canned response that may be served at most `n` times.
    pub fn expect_n(
        &self,
        fragment: impl Into<String>,
        response: impl Into<String>,
        n: u32,
    ) -> &Self {
        self.expectations.lock().unwrap().push(Expectation {
            fragment: fragment.into(),
            response: response.into(),
            remaining: Some(n),
        });
        self
    }
}

#[async_trait]
impl Prompter for MockPrompter {
    async fn complete(
        &self,
        _system: &str,
        user: &str,
        _cache_anchors: &[CacheAnchor],
    ) -> Result<Completion, PrompterError> {
        let mut expectations = self.expectations.lock().unwrap();

        let mut matched_index: Option<usize> = None;
        for (idx, exp) in expectations.iter().enumerate() {
            if user.contains(&exp.fragment) {
                matched_index = Some(idx);
                break;
            }
        }

        let idx = matched_index.ok_or_else(|| {
            PrompterError::UnexpectedPrompt(format!(
                "no registered fragment matched user prompt (length {} chars)",
                user.len()
            ))
        })?;

        let exp = &mut expectations[idx];
        let response = exp.response.clone();

        if let Some(remaining) = exp.remaining.as_mut() {
            *remaining = remaining.saturating_sub(1);
            if *remaining == 0 {
                expectations.remove(idx);
            }
        }

        Ok(Completion {
            text: response,
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_fragment_returns_response() {
        let mock = MockPrompter::new();
        mock.expect("alpha", "response-a");
        mock.expect("beta", "response-b");

        let r1 =
            pollster::block_on(mock.complete("sys", "user mentions alpha somewhere", &[])).unwrap();
        assert_eq!(r1.text, "response-a");

        let r2 = pollster::block_on(mock.complete("sys", "user mentions beta", &[])).unwrap();
        assert_eq!(r2.text, "response-b");
    }

    #[test]
    fn unmatched_prompt_errors() {
        let mock = MockPrompter::new();
        mock.expect("alpha", "ignored");

        let err = pollster::block_on(mock.complete("sys", "no fragments here", &[])).unwrap_err();
        assert!(matches!(err, PrompterError::UnexpectedPrompt(_)));
    }

    #[test]
    fn expect_n_caps_uses() {
        let mock = MockPrompter::new();
        mock.expect_n("foo", "bar", 1);

        pollster::block_on(mock.complete("sys", "foo present", &[])).unwrap();
        let err = pollster::block_on(mock.complete("sys", "foo present again", &[])).unwrap_err();
        assert!(matches!(err, PrompterError::UnexpectedPrompt(_)));
    }
}
