//! [`Prompter`] trait — the seam between the loop and any LLM provider.
//!
//! Implementors live in sibling crates (`wos-synth-anthropic`,
//! `wos-synth-mock`). The loop depends only on this trait.

use async_trait::async_trait;

/// Abstracts a single-turn LLM completion call.
///
/// Implementors handle transport, auth, retries, and (where supported)
/// prompt caching. The loop hands them a system prompt, a user prompt, and a
/// list of cache anchors; it expects back the model's text response and
/// token-usage metadata.
#[async_trait]
pub trait Prompter: Send + Sync {
    async fn complete(
        &self,
        system: &str,
        user: &str,
        cache_anchors: &[CacheAnchor],
    ) -> Result<Completion, PrompterError>;
}

/// A piece of context the provider may cache across calls.
///
/// Order matters: cache anchors are presented to providers (e.g., Anthropic's
/// prompt-caching API) in the order given. Stable, large content first.
#[derive(Debug, Clone)]
pub struct CacheAnchor {
    /// Human-readable label (e.g., `"kernel-schema"`, `"kernel-bluf"`).
    pub name: &'static str,
    /// The cacheable content.
    pub content: String,
}

/// Result of a successful prompter call.
#[derive(Debug, Clone)]
pub struct Completion {
    /// The model's text response (stripped of any provider-side wrapping).
    pub text: String,
    /// Tokens consumed reading the prompt (excludes cache reads).
    pub input_tokens: u64,
    /// Tokens generated.
    pub output_tokens: u64,
    /// Tokens served from prompt cache (subset of input).
    pub cache_read_tokens: u64,
}

/// Failure modes a prompter can surface.
#[derive(Debug, thiserror::Error)]
pub enum PrompterError {
    /// Transport or HTTP-level failure (network, timeout, 5xx).
    #[error("transport error: {0}")]
    Transport(String),

    /// Provider rejected the request (auth, rate limit, 4xx).
    #[error("provider rejected request: {0}")]
    ProviderRejected(String),

    /// Mock prompter received a prompt it has no canned response for.
    #[error("mock prompter received an unexpected prompt: {0}")]
    UnexpectedPrompt(String),

    /// Provider succeeded but returned an empty or malformed body.
    #[error("provider returned empty or malformed response: {0}")]
    EmptyResponse(String),
}
