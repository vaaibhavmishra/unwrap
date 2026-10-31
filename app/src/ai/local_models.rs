use ai::LLMProvider;

pub(super) const DEFAULT_ANTHROPIC_MODEL: &str = "claude-sonnet-4-5-20250929";
pub(super) const DEFAULT_OPENAI_MODEL: &str = "gpt-4.1";
pub(super) const DEFAULT_GOOGLE_MODEL: &str = "gemini-2.5-flash";
pub(super) const DEFAULT_OPENROUTER_MODEL: &str = "anthropic/claude-sonnet-4.5";

/// Local choices remain available without a Warp account or a server catalog.
pub(super) const LOCAL_MODELS: &[(&str, &str, LLMProvider)] = &[
    (
        DEFAULT_ANTHROPIC_MODEL,
        "Claude Sonnet 4.5",
        LLMProvider::Anthropic,
    ),
    (
        "claude-haiku-4-5-20251001",
        "Claude Haiku 4.5",
        LLMProvider::Anthropic,
    ),
    (DEFAULT_OPENAI_MODEL, "GPT-4.1", LLMProvider::OpenAI),
    ("gpt-4o", "GPT-4o", LLMProvider::OpenAI),
    (
        DEFAULT_GOOGLE_MODEL,
        "Gemini 2.5 Flash",
        LLMProvider::Google,
    ),
    ("gemini-2.5-pro", "Gemini 2.5 Pro", LLMProvider::Google),
    (
        "openrouter/anthropic/claude-sonnet-4.5",
        "Claude Sonnet 4.5 (OpenRouter)",
        LLMProvider::Unknown,
    ),
    (
        "openrouter/openai/gpt-4.1",
        "GPT-4.1 (OpenRouter)",
        LLMProvider::Unknown,
    ),
];
