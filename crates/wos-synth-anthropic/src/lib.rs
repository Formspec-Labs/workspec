//! Anthropic-API [`Prompter`] implementation.
//!
//! Crate-level isolation: this is the only crate in the synth family that
//! pulls `anthropic-sdk`, `reqwest`, and `tokio` into its dependency graph.
//! `wos-synth-core` stays network-free.
//!
//! Today the implementation uses the streaming-callback `anthropic-sdk` 0.1.x
//! API, mirroring the spike. Prompt caching is not yet wired through this SDK
//! and is tracked as future work — the [`CacheAnchor`] data is accepted but
//! folded into the system prompt verbatim. Once the SDK exposes cache control
//! we'll route anchors through it without a public API change.

use std::sync::{Arc, Mutex};

use anthropic_sdk::Client;
use async_trait::async_trait;
use serde_json::json;
use wos_synth_core::{CacheAnchor, Completion, Prompter, PrompterError};

/// Default Anthropic model id for synthesis.
///
/// `claude-opus-4-7` is the current strongest option for instruction-following
/// on long structured prompts. Override via [`AnthropicPrompter::with_model`].
pub const DEFAULT_MODEL: &str = "claude-opus-4-7";

/// Default max output tokens per call.
pub const DEFAULT_MAX_TOKENS: i32 = 4096;

/// Anthropic-backed [`Prompter`].
pub struct AnthropicPrompter {
    api_key: String,
    model: String,
    max_tokens: i32,
}

impl AnthropicPrompter {
    /// Construct from an explicit API key (caller is responsible for not
    /// committing the key).
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: DEFAULT_MODEL.to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: i32) -> Self {
        self.max_tokens = max_tokens;
        self
    }
}

#[async_trait]
impl Prompter for AnthropicPrompter {
    async fn complete(
        &self,
        system: &str,
        user: &str,
        cache_anchors: &[CacheAnchor],
    ) -> Result<Completion, PrompterError> {
        // Compose the effective system prompt: caller's system + anchors.
        // When the SDK gains cache_control support, this concatenation is the
        // line that splits up — anchors will move to per-block cache_control
        // headers and stay out of the regular prompt billing.
        let composed_system = compose_system_prompt(system, cache_anchors);

        let collected: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
        let collector = Arc::clone(&collected);

        Client::new()
            .auth(&self.api_key)
            .model(&self.model)
            .max_tokens(self.max_tokens)
            .temperature(0.0)
            .system(&composed_system)
            .messages(&json!([
                { "role": "user", "content": user }
            ]))
            .build()
            .map_err(|e| PrompterError::ProviderRejected(e.to_string()))?
            .execute(move |chunk| {
                let collector = Arc::clone(&collector);
                async move {
                    collector.lock().unwrap().push_str(&chunk);
                }
            })
            .await
            .map_err(|e| PrompterError::Transport(e.to_string()))?;

        let text = Arc::try_unwrap(collected)
            .map_err(|_| {
                PrompterError::EmptyResponse(
                    "callback retained collector reference".into(),
                )
            })?
            .into_inner()
            .unwrap();

        if text.trim().is_empty() {
            return Err(PrompterError::EmptyResponse(
                "Anthropic returned no text".into(),
            ));
        }

        // The 0.1.x SDK doesn't surface usage from the streaming callback.
        // Token counts default to 0 here; once the SDK exposes usage metadata
        // we'll wire it through. Trace consumers should treat 0 as "unknown."
        Ok(Completion {
            text,
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: 0,
        })
    }
}

fn compose_system_prompt(system: &str, anchors: &[CacheAnchor]) -> String {
    if anchors.is_empty() {
        return system.to_string();
    }

    let anchor_blocks = anchors
        .iter()
        .map(|a| format!("<context name=\"{name}\">\n{content}\n</context>", name = a.name, content = a.content))
        .collect::<Vec<_>>()
        .join("\n\n");

    format!("{system}\n\n{anchor_blocks}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_system_prompt_passes_through_when_empty() {
        let out = compose_system_prompt("base", &[]);
        assert_eq!(out, "base");
    }

    #[test]
    fn compose_system_prompt_wraps_anchors_in_named_blocks() {
        let anchors = vec![
            CacheAnchor { name: "schema", content: "{}".into() },
            CacheAnchor { name: "spec", content: "lorem".into() },
        ];
        let out = compose_system_prompt("base", &anchors);
        assert!(out.contains("base"));
        assert!(out.contains("<context name=\"schema\">"));
        assert!(out.contains("<context name=\"spec\">"));
        assert!(out.contains("lorem"));
    }
}
