//! The curated set of goose providers Lazyboy exposes in its settings UI.
//!
//! Goose embeds a much larger declarative catalog, but the settings
//! surface only needs the providers a single-tenant operator is likely to
//! use, each with the exact `*_API_KEY` env var goose reads (taken from
//! goose 1.37.0's catalog) and a few current model ids as suggestions.
//! The list is a suggestion, not a constraint: `requires_key` drives
//! whether the UI demands a key, and the model field is free-form so a
//! provider's newer model still works without a catalog bump.

/// One provider entry: how the UI labels it, the env var its key is
/// injected as when launching `goose serve`, whether a key is required at
/// all (a local Ollama needs none), and suggested models.
pub struct ProviderSpec {
    pub id: &'static str,
    pub display_name: &'static str,
    pub key_env: &'static str,
    pub requires_key: bool,
    pub models: &'static [&'static str],
}

/// The catalog, ordered by how commonly a Lazyboy operator reaches for
/// each. `id` is goose's provider id (the value written as
/// `GOOSE_PROVIDER`); `key_env` is the exact var goose reads the secret
/// from, so the launcher sets it verbatim.
pub const PROVIDERS: &[ProviderSpec] = &[
    ProviderSpec {
        id: "anthropic",
        display_name: "Anthropic (Claude)",
        key_env: "ANTHROPIC_API_KEY",
        requires_key: true,
        models: &[
            "claude-opus-4-20250514",
            "claude-sonnet-4-20250514",
            "claude-haiku-4-20250514",
        ],
    },
    ProviderSpec {
        id: "openai",
        display_name: "OpenAI",
        key_env: "OPENAI_API_KEY",
        requires_key: true,
        models: &["gpt-4o", "gpt-4o-mini", "o3-mini"],
    },
    ProviderSpec {
        id: "google",
        display_name: "Google (Gemini)",
        key_env: "GOOGLE_API_KEY",
        requires_key: true,
        models: &["gemini-2.5-pro", "gemini-2.5-flash"],
    },
    ProviderSpec {
        id: "groq",
        display_name: "Groq",
        key_env: "GROQ_API_KEY",
        requires_key: true,
        models: &["llama-3.3-70b-versatile", "gpt-oss-120b"],
    },
    ProviderSpec {
        id: "openrouter",
        display_name: "OpenRouter",
        key_env: "OPENROUTER_API_KEY",
        requires_key: true,
        models: &["anthropic/claude-sonnet-4", "openai/gpt-4o"],
    },
    ProviderSpec {
        id: "deepseek",
        display_name: "DeepSeek",
        key_env: "DEEPSEEK_API_KEY",
        requires_key: true,
        models: &["deepseek-chat", "deepseek-reasoner"],
    },
    ProviderSpec {
        id: "xai",
        display_name: "xAI (Grok)",
        key_env: "XAI_API_KEY",
        requires_key: true,
        models: &["grok-4", "grok-3"],
    },
    ProviderSpec {
        id: "ollama",
        display_name: "Ollama (local)",
        key_env: "OLLAMA_HOST",
        requires_key: false,
        models: &["llama3.3", "qwen2.5"],
    },
];

/// Look up a provider by its goose id, or `None` if it is not one Lazyboy
/// exposes — the boundary that rejects a settings write naming a provider
/// the launcher would not know how to configure.
pub fn find(id: &str) -> Option<&'static ProviderSpec> {
    PROVIDERS.iter().find(|p| p.id == id)
}
