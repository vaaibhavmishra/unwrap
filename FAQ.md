# Frequently Asked Questions

This FAQ covers common questions about Unwrap, building from source, and using the built-in AI agent.

---

### What is Unwrap?

Unwrap is a fast, native terminal emulator built in Rust. It provides a GPU-accelerated canvas, modern block-based command execution, and a local AI coding assistant without requiring accounts, logins, telemetry, or proprietary cloud backends.

---

### How is Unwrap different from other terminals?

1. **No Account or Sign-in Required**: Unwrap is completely usable out-of-the-box without signing in or registering.
2. **Zero Telemetry**: No crash reporting to external servers, no tracking, and no session sharing over the cloud.
3. **Direct BYOK (Bring Your Own Key) AI**: The built-in coding agent connects directly from your machine to Anthropic Claude, OpenAI, Google Gemini, OpenRouter, or local servers (Ollama, vLLM). No intermediary proxy servers are involved.
4. **Dual Front-Ends**: Runs either as a rich GPU desktop application (`Unwrap.app`) or as a headless console TUI (`warp_tui`).

---

### How do I configure AI models?

Open **Settings** (`Cmd+,` on macOS), navigate to **Agent → API Keys**, and supply your API key for Google Gemini, Anthropic, OpenAI, or OpenRouter. You can also configure custom OpenAI-compatible endpoints for local LLMs like Ollama.

---

### How do I build and run Unwrap from source?

```bash
# Bootstrap dependencies
./script/bootstrap

# Run the GUI desktop app
./script/run

# Run the console TUI
./script/run-tui
```

---

### How do I package Unwrap as a macOS `.app` bundle?

Run the bundle script:

```bash
./script/macos/bundle --channel oss --nouniversal --nosign
```

The compiled application will be generated at `target/release-lto/bundle/osx/Unwrap.app`.

---

### What license is Unwrap distributed under?

The custom UI framework (`crates/warpui` and `crates/warpui_core`) is licensed under the [MIT License](LICENSE-MIT). The rest of the terminal code is licensed under the [GNU Affero General Public License v3 (AGPL-3.0)](LICENSE-AGPL).
