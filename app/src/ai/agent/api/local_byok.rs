use std::sync::Arc;
use std::time::SystemTime;

use serde_json::{Value, json};
use uuid::Uuid;
use warp_multi_agent_api as api;
use warp_multi_agent_api::client_action::{Action, AddMessagesToTask, CreateTask};
use warp_multi_agent_api::message::tool_call::{ApplyFileDiffs, ReadFiles, RunShellCommand, Tool};
use warp_multi_agent_api::message::{AgentOutput, Message as MessageKind, ToolCall};
use warp_multi_agent_api::response_event::{StreamFinished, StreamInit, stream_finished};
use warp_multi_agent_api::{ClientAction, Message, ResponseEvent, response_event};

use super::convert_to::convert_input;
use super::{ConvertToAPITypeError, RequestParams, ResponseStream};
use crate::ai::local_models::{
    DEFAULT_ANTHROPIC_MODEL, DEFAULT_GOOGLE_MODEL, DEFAULT_OPENAI_MODEL, DEFAULT_OPENROUTER_MODEL,
};
use crate::server::server_api::AIApiError;

#[path = "local_byok_stream.rs"]
mod stream;
use stream::{StreamProtocol, consume_stream};

const SYSTEM_PROMPT: &str = "\
You are Unwrap Agent, an expert AI coding and system assistant embedded directly in Unwrap terminal on macOS/Linux.
You have tools to execute shell commands, read files, and apply file modifications.

Guidelines:
- When asked to inspect a project or answer a question about code or the system, use your tools (run_shell_command, read_files, apply_file_diffs).
- Be concise, direct, and actionable. Avoid unnecessary fluff or repeating tool results.
- When executing commands, prefer non-destructive and relevant commands.
- When editing files, propose precise search and replace blocks or full new file contents.";

#[derive(Clone)]
enum ByokProvider {
    Anthropic {
        api_key: String,
        base_url: Option<String>,
        model: String,
    },
    OpenAiChatCompletions {
        api_key: String,
        base_url: String,
        model: String,
    },
    OpenAiResponses {
        api_key: String,
        base_url: String,
        model: String,
    },
}

fn custom_provider(
    provider: &api::request::settings::custom_model_providers::CustomModelProvider,
    model: &api::request::settings::custom_model_providers::CustomModel,
) -> anyhow::Result<ByokProvider> {
    use api::request::settings::custom_model_providers::CustomEndpointSchema;

    anyhow::ensure!(
        !provider.base_url.trim().is_empty()
            && !provider.api_key.trim().is_empty()
            && !model.slug.trim().is_empty(),
        "Selected custom model is not configured. Check Settings > Agent > API Keys."
    );
    let base_url = provider.base_url.trim().trim_end_matches('/').to_string();
    let api_key = provider.api_key.clone();
    let model = model.slug.clone();
    Ok(match CustomEndpointSchema::try_from(provider.schema)? {
        CustomEndpointSchema::AnthropicMessages => ByokProvider::Anthropic {
            api_key,
            base_url: Some(base_url),
            model,
        },
        CustomEndpointSchema::OpenaiResponses => ByokProvider::OpenAiResponses {
            api_key,
            base_url,
            model,
        },
        CustomEndpointSchema::OpenaiChatCompletions => ByokProvider::OpenAiChatCompletions {
            api_key,
            base_url,
            model,
        },
    })
}

fn resolve_provider(params: &RequestParams) -> anyhow::Result<Option<ByokProvider>> {
    let api_keys = params.api_keys.as_ref();
    let anthropic_key = api_keys
        .map(|k| k.anthropic.trim())
        .filter(|s| !s.is_empty());
    let openai_key = api_keys.map(|k| k.openai.trim()).filter(|s| !s.is_empty());
    let google_key = api_keys.map(|k| k.google.trim()).filter(|s| !s.is_empty());
    let open_router_key = api_keys
        .map(|k| k.open_router.trim())
        .filter(|s| !s.is_empty());
    let model = params.model.as_str().trim();
    let automatic = model == "auto";

    if !automatic && let Some(custom) = &params.custom_model_providers {
        let mut matches = custom.providers.iter().flat_map(|provider| {
            provider
                .models
                .iter()
                .filter(move |m| m.config_key == model)
                .map(move |m| (provider, m))
        });
        if let Some((provider, model)) = matches.next() {
            anyhow::ensure!(
                matches.next().is_none(),
                "Selected custom model has an ambiguous configuration."
            );
            return custom_provider(provider, model).map(Some);
        }
    }

    let chat = |key: &str, base_url: &str, model: &str| ByokProvider::OpenAiChatCompletions {
        api_key: key.to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
    };
    let missing_key = || {
        anyhow::anyhow!(
            "The selected model has no API key. Configure its provider in Settings > Agent > API Keys, or select another model."
        )
    };
    // Qualified router IDs must be resolved before inspecting the underlying model name.
    if let Some(model) = model.strip_prefix("openrouter/") {
        anyhow::ensure!(!model.is_empty(), "Select an OpenRouter model.");
        return Ok(Some(chat(
            open_router_key.ok_or_else(missing_key)?,
            "https://openrouter.ai/api/v1",
            model,
        )));
    }
    if model.starts_with("claude-") || (automatic && anthropic_key.is_some()) {
        return Ok(Some(ByokProvider::Anthropic {
            api_key: anthropic_key.ok_or_else(missing_key)?.to_string(),
            base_url: None,
            model: if automatic {
                DEFAULT_ANTHROPIC_MODEL
            } else {
                model
            }
            .to_string(),
        }));
    }
    if model.starts_with("gemini-") || (automatic && google_key.is_some()) {
        return Ok(Some(chat(
            google_key.ok_or_else(missing_key)?,
            "https://generativelanguage.googleapis.com/v1beta/openai",
            if automatic {
                DEFAULT_GOOGLE_MODEL
            } else {
                model
            },
        )));
    }
    if model.starts_with("gpt-")
        || ["o1", "o3", "o4"]
            .iter()
            .any(|prefix| model == *prefix || model.starts_with(&format!("{prefix}-")))
        || (automatic && openai_key.is_some())
    {
        return Ok(Some(chat(
            openai_key.ok_or_else(missing_key)?,
            "https://api.openai.com/v1",
            if automatic {
                DEFAULT_OPENAI_MODEL
            } else {
                model
            },
        )));
    }
    if automatic {
        if let Some(key) = open_router_key {
            return Ok(Some(chat(
                key,
                "https://openrouter.ai/api/v1",
                DEFAULT_OPENROUTER_MODEL,
            )));
        }
        if let Some(custom) = &params.custom_model_providers {
            for provider in &custom.providers {
                for model in &provider.models {
                    if let Ok(provider) = custom_provider(provider, model) {
                        return Ok(Some(provider));
                    }
                }
            }
        }
        return Ok(None);
    }
    anyhow::bail!(
        "The selected model is unavailable. Restore its custom endpoint or choose a configured model in the model picker."
    )
}

fn convert_input_tool_result_to_message_result(
    r: &api::request::input::tool_call_result::Result,
) -> Option<api::message::tool_call_result::Result> {
    match r {
        api::request::input::tool_call_result::Result::RunShellCommand(cmd) => Some(
            api::message::tool_call_result::Result::RunShellCommand(cmd.clone()),
        ),
        api::request::input::tool_call_result::Result::ReadFiles(rf) => Some(
            api::message::tool_call_result::Result::ReadFiles(rf.clone()),
        ),
        api::request::input::tool_call_result::Result::ApplyFileDiffs(afd) => Some(
            api::message::tool_call_result::Result::ApplyFileDiffs(afd.clone()),
        ),
        _ => None,
    }
}

pub(crate) async fn generate_local_byok_output(
    mut params: RequestParams,
    cancellation_rx: futures::channel::oneshot::Receiver<()>,
) -> Result<ResponseStream, ConvertToAPITypeError> {
    let (tx, rx) = async_channel::unbounded();

    let request_id = Uuid::new_v4().to_string();
    let conversation_id = params
        .conversation_token
        .as_ref()
        .map(|t| t.as_str().to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let run_id = Uuid::new_v4().to_string();

    let task_id = params
        .tasks
        .last()
        .map(|t| t.id.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let is_new_task = params.tasks.is_empty();

    let provider = resolve_provider(&params);
    let converted_input = convert_input(std::mem::take(&mut params.input))?;

    tokio::spawn(async move {
        tokio::select! {
            _ = cancellation_rx => {}
            _ = async {
                let init_event = ResponseEvent {
                    r#type: Some(response_event::Type::Init(StreamInit {
                        conversation_id: conversation_id.clone(),
                        request_id: request_id.clone(),
                        run_id: run_id.clone(),
                    })),
                };
                if tx.send(Ok(init_event)).await.is_err() {
                    return;
                }

                if is_new_task {
                    let create_task_event = ResponseEvent {
                        r#type: Some(response_event::Type::ClientActions(
                            response_event::ClientActions {
                                actions: vec![ClientAction {
                                    action: Some(Action::CreateTask(CreateTask {
                                        task: Some(api::Task {
                                            id: task_id.clone(),
                                            ..Default::default()
                                        }),
                                    })),
                                }],
                            },
                        )),
                    };
                    if tx.send(Ok(create_task_event)).await.is_err() {
                        return;
                    }
                }

                let mut input_messages = Vec::new();
                if let Some(ref input_type) = converted_input.r#type {
                    match input_type {
                        api::request::input::Type::UserInputs(inputs) => {
                            for inp in &inputs.inputs {
                                if let Some(ref item) = inp.input {
                                    match item {
                                        api::request::input::user_inputs::user_input::Input::UserQuery(uq) => {
                                            input_messages.push(Message {
                                                id: Uuid::new_v4().to_string(),
                                                task_id: task_id.clone(),
                                                request_id: request_id.clone(),
                                                timestamp: Some(prost_types::Timestamp::from(SystemTime::now())),
                                                message: Some(MessageKind::UserQuery(api::message::UserQuery {
                                                    query: uq.query.clone(),
                                                    ..Default::default()
                                                })),
                                                ..Default::default()
                                            });
                                        }
                                        api::request::input::user_inputs::user_input::Input::CliAgentUserQuery(ca) => {
                                            if let Some(ref uq) = ca.user_query {
                                                input_messages.push(Message {
                                                    id: Uuid::new_v4().to_string(),
                                                    task_id: task_id.clone(),
                                                    request_id: request_id.clone(),
                                                    timestamp: Some(prost_types::Timestamp::from(SystemTime::now())),
                                                    message: Some(MessageKind::UserQuery(api::message::UserQuery {
                                                        query: uq.query.clone(),
                                                        ..Default::default()
                                                    })),
                                                    ..Default::default()
                                                });
                                            }
                                        }
                                        api::request::input::user_inputs::user_input::Input::ToolCallResult(tcr) => {
                                            input_messages.push(Message {
                                                id: Uuid::new_v4().to_string(),
                                                task_id: task_id.clone(),
                                                request_id: request_id.clone(),
                                                timestamp: Some(prost_types::Timestamp::from(SystemTime::now())),
                                                message: Some(MessageKind::ToolCallResult(api::message::ToolCallResult {
                                                    tool_call_id: tcr.tool_call_id.clone(),
                                                    result: tcr.result.as_ref().and_then(convert_input_tool_result_to_message_result),
                                                    context: None,
                                                })),
                                                ..Default::default()
                                            });
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        api::request::input::Type::CreateNewProject(p) => {
                            input_messages.push(Message {
                                id: Uuid::new_v4().to_string(),
                                task_id: task_id.clone(),
                                request_id: request_id.clone(),
                                timestamp: Some(prost_types::Timestamp::from(SystemTime::now())),
                                message: Some(MessageKind::UserQuery(api::message::UserQuery {
                                    query: p.query.clone(),
                                    ..Default::default()
                                })),
                                ..Default::default()
                            });
                        }
                        _ => {}
                    }
                }

                if !input_messages.is_empty() {
                    let input_action = ClientAction {
                        action: Some(Action::AddMessagesToTask(AddMessagesToTask {
                            task_id: task_id.clone(),
                            messages: input_messages,
                        })),
                    };
                    if tx.send(Ok(ResponseEvent {
                        r#type: Some(response_event::Type::ClientActions(
                            response_event::ClientActions {
                                actions: vec![input_action],
                            },
                        )),
                    })).await.is_err() {
                        return;
                    }
                }

                if matches!(provider, Ok(None)) {
                    let message_id = Uuid::new_v4().to_string();
                    let welcome_text = "Welcome to Unwrap Agent!\n\n\
                        To start using Agent Mode, please configure your API key in **Settings > Agent > API Keys**.\n\n\
                        You can add:\n\
                        - **Google Gemini** (e.g. Gemini 2.5 Flash, Gemini 2.5 Pro) in the Google field\n\
                        - **Anthropic Claude** (e.g. Claude Sonnet 4.5, Claude Haiku 4.5)\n\
                        - **OpenAI** (e.g. GPT-4o, o3-mini)\n\
                        - **OpenRouter** or any **Custom Endpoint** (local Ollama, vLLM, LiteLLM)\n\n\
                        Once configured, your agent runs directly with external model providers.";

                    let msg_action = ClientAction {
                        action: Some(Action::AddMessagesToTask(AddMessagesToTask {
                            task_id: task_id.clone(),
                            messages: vec![Message {
                                id: message_id,
                                task_id: task_id.clone(),
                                request_id: request_id.clone(),
                                timestamp: Some(prost_types::Timestamp::from(SystemTime::now())),
                                message: Some(MessageKind::AgentOutput(AgentOutput {
                                    text: welcome_text.to_string(),
                                })),
                                ..Default::default()
                            }],
                        })),
                    };

                    let _ = tx
                        .send(Ok(ResponseEvent {
                            r#type: Some(response_event::Type::ClientActions(
                                response_event::ClientActions {
                                    actions: vec![msg_action],
                                },
                            )),
                        }))
                        .await;

                    let _ = tx
                        .send(Ok(ResponseEvent {
                            r#type: Some(response_event::Type::Finished(StreamFinished {
                                reason: Some(stream_finished::Reason::Done(stream_finished::Done {})),
                                ..Default::default()
                            })),
                        }))
                        .await;
                    return;
                };

                let result = match provider {
                    Err(error) => Err(error),
                    Ok(None) => unreachable!("unconfigured auto handled above"),
                    Ok(Some(ByokProvider::Anthropic { api_key, base_url, model })) => {
                        run_anthropic_stream(
                            &api_key,
                            base_url.as_deref(),
                            &model,
                            &task_id,
                            &request_id,
                            &params.tasks,
                            &converted_input,
                            &tx,
                        )
                        .await
                    }
                    Ok(Some(ByokProvider::OpenAiChatCompletions {
                        api_key,
                        base_url,
                        model,
                    })) => {
                        run_openai_stream(
                            &api_key,
                            &base_url,
                            &model,
                            false,
                            &task_id,
                            &request_id,
                            &params.tasks,
                            &converted_input,
                            &tx,
                        )
                        .await
                    }
                    Ok(Some(ByokProvider::OpenAiResponses {
                        api_key,
                        base_url,
                        model,
                    })) => {
                        run_openai_stream(
                            &api_key,
                            &base_url,
                            &model,
                            true,
                            &task_id,
                            &request_id,
                            &params.tasks,
                            &converted_input,
                            &tx,
                        )
                        .await
                    }
                };

                match result {
                    Ok(()) => {
                        let _ = tx
                            .send(Ok(ResponseEvent {
                                r#type: Some(response_event::Type::Finished(StreamFinished {
                                    reason: Some(stream_finished::Reason::Done(stream_finished::Done {})),
                                    ..Default::default()
                                })),
                            }))
                            .await;
                    }
                    Err(e) => {
                        let error_mid = Uuid::new_v4().to_string();
                        let err_msg = ClientAction {
                            action: Some(Action::AddMessagesToTask(AddMessagesToTask {
                                task_id: task_id.clone(),
                                messages: vec![Message {
                                    id: error_mid,
                                    task_id: task_id.clone(),
                                    request_id: request_id.clone(),
                                    timestamp: Some(prost_types::Timestamp::from(SystemTime::now())),
                                    message: Some(MessageKind::AgentOutput(AgentOutput {
                                        text: format!("⚠️ Agent Error: {e:#}"),
                                    })),
                                    ..Default::default()
                                }],
                            })),
                        };
                        let _ = tx
                            .send(Ok(ResponseEvent {
                                r#type: Some(response_event::Type::ClientActions(
                                    response_event::ClientActions {
                                        actions: vec![err_msg],
                                    },
                                )),
                            }))
                            .await;
                        let _ = tx
                            .send(Ok(ResponseEvent {
                                r#type: Some(response_event::Type::Finished(StreamFinished {
                                    reason: Some(stream_finished::Reason::InternalError(
                                        stream_finished::InternalError {
                                            message: format!("{e:#}"),
                                        },
                                    )),
                                    ..Default::default()
                                })),
                            }))
                            .await;
                        let _ = tx
                            .send(Err(Arc::new(AIApiError::Other(anyhow::anyhow!("{e:#}")))))
                            .await;
                    }
                }
            } => {}
        }
    });

    Ok(Box::pin(rx))
}

fn get_openai_tools() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "run_shell_command",
                "description": "Execute a bash shell command in the user's terminal session",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The command line string to execute."
                        }
                    },
                    "required": ["command"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "read_files",
                "description": "Read the contents of specified files in the workspace",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "file_paths": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "The paths of the files to read."
                        }
                    },
                    "required": ["file_paths"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "apply_file_diffs",
                "description": "Apply modifications or create new files in the workspace",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "summary": { "type": "string", "description": "Brief description of the changes" },
                        "diffs": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string" },
                                    "search": { "type": "string", "description": "Exact text to find" },
                                    "replace": { "type": "string", "description": "Replacement text" }
                                },
                                "required": ["file_path", "search", "replace"]
                            }
                        },
                        "new_files": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string" },
                                    "content": { "type": "string" },
                                    "allow_overwrite": { "type": "boolean", "description": "Explicitly authorize overwriting an existing file." }
                                },
                                "required": ["file_path", "content"]
                            }
                        }
                    }
                }
            }
        }
    ])
}

fn get_anthropic_tools() -> Value {
    json!([
        {
            "name": "run_shell_command",
            "description": "Execute a bash shell command in the user's terminal session",
            "input_schema": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The command line string to execute."
                    }
                },
                "required": ["command"]
            }
        },
        {
            "name": "read_files",
            "description": "Read the contents of specified files in the workspace",
            "input_schema": {
                "type": "object",
                "properties": {
                    "file_paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "The paths of the files to read."
                    }
                },
                "required": ["file_paths"]
            }
        },
        {
            "name": "apply_file_diffs",
            "description": "Apply modifications or create new files in the workspace",
            "input_schema": {
                "type": "object",
                "properties": {
                    "summary": { "type": "string", "description": "Brief description of the changes" },
                    "diffs": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "file_path": { "type": "string" },
                                "search": { "type": "string", "description": "Exact text to find" },
                                "replace": { "type": "string", "description": "Replacement text" }
                            },
                            "required": ["file_path", "search", "replace"]
                        }
                    },
                    "new_files": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "file_path": { "type": "string" },
                                "content": { "type": "string" },
                                "allow_overwrite": { "type": "boolean", "description": "Explicitly authorize overwriting an existing file." }
                            },
                            "required": ["file_path", "content"]
                        }
                    }
                }
            }
        }
    ])
}

fn format_shell_result(r: &api::RunShellCommandResult) -> String {
    if let Some(api::run_shell_command_result::Result::CommandFinished(ref cf)) = r.result {
        format!(
            "Command output (exit code {}):\n{}",
            cf.exit_code, cf.output
        )
    } else {
        #[allow(deprecated)]
        let out = &r.output;
        format!("Command output:\n{out}")
    }
}

fn format_read_files_result(rf: &api::ReadFilesResult) -> String {
    let mut s = String::new();
    if let Some(ref res) = rf.result {
        match res {
            api::read_files_result::Result::AnyFilesSuccess(any_files) => {
                for f in &any_files.files {
                    if let Some(ref c) = f.content {
                        match c {
                            api::any_file_content::Content::TextContent(tc) => {
                                s.push_str(&format!("File: {}\n{}\n", tc.file_path, tc.content));
                            }
                            api::any_file_content::Content::BinaryContent(bc) => {
                                s.push_str(&format!("File: {} (binary)\n", bc.file_path));
                            }
                        }
                    }
                }
            }
            api::read_files_result::Result::TextFilesSuccess(text_files) => {
                for f in &text_files.files {
                    s.push_str(&format!("File: {}\n{}\n", f.file_path, f.content));
                }
            }
            api::read_files_result::Result::Error(err) => {
                s.push_str(&format!("Error reading files: {}", err.message));
            }
        }
    }
    if s.is_empty() {
        s = "Files read successfully.".to_string();
    }
    s
}

fn format_apply_file_diffs_result(afd: &api::ApplyFileDiffsResult) -> String {
    match &afd.result {
        Some(api::apply_file_diffs_result::Result::Success(s)) => {
            let mut out = "File changes applied successfully.".to_string();
            for f in &s.updated_files_v2 {
                if let Some(ref file) = f.file {
                    out.push_str(&format!("\nUpdated: {}", file.file_path));
                }
            }
            for f in &s.deleted_files {
                out.push_str(&format!("\nDeleted: {}", f.file_path));
            }
            out
        }
        Some(api::apply_file_diffs_result::Result::Error(err)) => {
            format!("Error applying file diffs: {}", err.message)
        }
        None => "File changes were cancelled or not applied.".to_string(),
    }
}

fn format_system_prompt_with_context(context: Option<&api::InputContext>) -> String {
    let mut prompt = SYSTEM_PROMPT.to_string();
    if let Some(ctx) = context {
        let mut context_details = Vec::new();
        if let Some(ref dir) = ctx.directory
            && !dir.pwd.is_empty()
        {
            context_details.push(format!("Working Directory: {}", dir.pwd));
        }
        if let Some(ref os) = ctx.operating_system
            && !os.platform.is_empty()
        {
            context_details.push(format!("OS: {} {}", os.platform, os.distribution));
        }
        if let Some(ref shell) = ctx.shell
            && !shell.name.is_empty()
        {
            context_details.push(format!("Shell: {}", shell.name));
        }
        for sel in &ctx.selected_text {
            if !sel.text.is_empty() {
                context_details.push(format!("Selected text:\n```\n{}\n```", sel.text));
            }
        }
        for rules in &ctx.project_rules {
            for file in &rules.active_rule_files {
                if !file.content.trim().is_empty() {
                    context_details.push(format!(
                        "Project Rule ({}):\n{}",
                        file.file_path, file.content
                    ));
                }
            }
        }
        if !context_details.is_empty() {
            prompt.push_str("\n\nCurrent Workspace Context:\n");
            prompt.push_str(&context_details.join("\n"));
        }
    }
    prompt
}

fn extract_messages_for_llm(
    tasks: &[api::Task],
    current_input: &api::request::Input,
) -> Vec<(String, String)> {
    let mut messages: Vec<(String, String)> = Vec::new();

    for task in tasks {
        for msg in &task.messages {
            if let Some(ref m) = msg.message {
                match m {
                    MessageKind::UserQuery(uq) => {
                        messages.push(("user".to_string(), uq.query.clone()));
                    }
                    MessageKind::AgentOutput(ao) => {
                        messages.push(("assistant".to_string(), ao.text.clone()));
                    }
                    MessageKind::ToolCall(tc) => {
                        let desc = match &tc.tool {
                            Some(Tool::RunShellCommand(cmd)) => {
                                format!("[Executed command: {}]", cmd.command)
                            }
                            Some(Tool::ReadFiles(rf)) => format!(
                                "[Read files: {:?}]",
                                rf.files.iter().map(|f| &f.name).collect::<Vec<_>>()
                            ),
                            Some(Tool::ApplyFileDiffs(afd)) => {
                                format!("[Applied diffs: {}]", afd.summary)
                            }
                            _ => "[Tool call]".to_string(),
                        };
                        messages.push(("assistant".to_string(), desc));
                    }
                    MessageKind::ToolCallResult(tcr) => {
                        let res_str = match &tcr.result {
                            Some(api::message::tool_call_result::Result::RunShellCommand(r)) => {
                                format_shell_result(r)
                            }
                            Some(api::message::tool_call_result::Result::ReadFiles(rf)) => {
                                format_read_files_result(rf)
                            }
                            Some(api::message::tool_call_result::Result::ApplyFileDiffs(afd)) => {
                                format_apply_file_diffs_result(afd)
                            }
                            _ => "Tool execution completed.".to_string(),
                        };
                        messages.push(("user".to_string(), res_str));
                    }
                    _ => {}
                }
            }
        }
    }

    if let Some(ref input_type) = current_input.r#type {
        match input_type {
            api::request::input::Type::UserInputs(inputs) => {
                for inp in &inputs.inputs {
                    if let Some(ref item) = inp.input {
                        match item {
                            api::request::input::user_inputs::user_input::Input::UserQuery(uq) => {
                                let mut query_text = uq.query.clone();
                                for (name, att) in &uq.referenced_attachments {
                                    match &att.value {
                                        Some(api::attachment::Value::PlainText(t)) => {
                                            query_text.push_str(&format!("\n[Attached Content {name}:\n{t}]"));
                                        }
                                        Some(api::attachment::Value::FilePathReference(fpr)) => {
                                            query_text.push_str(&format!("\n[Attached File {name}: {}]", fpr.file_path));
                                        }
                                        _ => {
                                            query_text.push_str(&format!("\n[Attached {name}]"));
                                        }
                                    }
                                }
                                messages.push(("user".to_string(), query_text));
                            }
                            api::request::input::user_inputs::user_input::Input::CliAgentUserQuery(ca) => {
                                if let Some(ref uq) = ca.user_query {
                                    let mut query_text = uq.query.clone();
                                    for (name, att) in &uq.referenced_attachments {
                                        match &att.value {
                                            Some(api::attachment::Value::PlainText(t)) => {
                                                query_text.push_str(&format!("\n[Attached Content {name}:\n{t}]"));
                                            }
                                            Some(api::attachment::Value::FilePathReference(fpr)) => {
                                                query_text.push_str(&format!("\n[Attached File {name}: {}]", fpr.file_path));
                                            }
                                            _ => {
                                                query_text.push_str(&format!("\n[Attached {name}]"));
                                            }
                                        }
                                    }
                                    messages.push(("user".to_string(), query_text));
                                }
                            }
                            api::request::input::user_inputs::user_input::Input::ToolCallResult(tcr) => {
                                let res_str = match &tcr.result {
                                    Some(api::request::input::tool_call_result::Result::RunShellCommand(r)) => {
                                        format_shell_result(r)
                                    }
                                    Some(api::request::input::tool_call_result::Result::ReadFiles(rf)) => {
                                        format_read_files_result(rf)
                                    }
                                    Some(api::request::input::tool_call_result::Result::ApplyFileDiffs(afd)) => {
                                        format_apply_file_diffs_result(afd)
                                    }
                                    _ => "Tool executed successfully.".to_string(),
                                };
                                messages.push(("user".to_string(), res_str));
                            }
                            _ => {}
                        }
                    }
                }
            }
            api::request::input::Type::CreateNewProject(p) => {
                messages.push(("user".to_string(), p.query.clone()));
            }
            _ => {}
        }
    }

    messages
}

fn required_string<'a>(value: &'a Value, field: &str) -> anyhow::Result<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Invalid or missing {field} in provider response"))
}

fn optional_array<'a>(value: &'a Value, field: &str) -> anyhow::Result<&'a [Value]> {
    match value.get(field) {
        None => Ok(&[]),
        Some(Value::Array(items)) => Ok(items),
        Some(_) => anyhow::bail!("Invalid {field}: expected an array"),
    }
}

fn parse_tool_call_args(
    tool_id: String,
    tool_name: &str,
    args_json: &str,
) -> anyhow::Result<MessageKind> {
    anyhow::ensure!(!tool_id.is_empty(), "Missing tool call ID");
    let args: Value = serde_json::from_str(args_json)?;
    anyhow::ensure!(args.is_object(), "Tool arguments must be a JSON object");
    let tool = match tool_name {
        "run_shell_command" => Tool::RunShellCommand(RunShellCommand {
            command: required_string(&args, "command")?.to_string(),
            ..Default::default()
        }),
        "read_files" => {
            let paths = args
                .get("file_paths")
                .and_then(Value::as_array)
                .ok_or_else(|| anyhow::anyhow!("Missing file_paths array"))?;
            let files = paths
                .iter()
                .map(|path| {
                    Ok(api::message::tool_call::read_files::File {
                        name: path
                            .as_str()
                            .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?
                            .to_string(),
                        line_ranges: vec![],
                    })
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            Tool::ReadFiles(ReadFiles { files })
        }
        "apply_file_diffs" => {
            let summary = if args.get("summary").is_some() {
                required_string(&args, "summary")?
            } else {
                "Edit files"
            }
            .to_string();
            let diffs = optional_array(&args, "diffs")?
                .iter()
                .map(|item| {
                    Ok(api::message::tool_call::apply_file_diffs::FileDiff {
                        file_path: required_string(item, "file_path")?.to_string(),
                        search: required_string(item, "search")?.to_string(),
                        replace: required_string(item, "replace")?.to_string(),
                    })
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            let new_files = optional_array(&args, "new_files")?
                .iter()
                .map(|item| {
                    let allow_overwrite = match item.get("allow_overwrite") {
                        None => false,
                        Some(Value::Bool(value)) => *value,
                        Some(_) => anyhow::bail!("Invalid allow_overwrite: expected a boolean"),
                    };
                    Ok(api::message::tool_call::apply_file_diffs::NewFile {
                        file_path: required_string(item, "file_path")?.to_string(),
                        content: required_string(item, "content")?.to_string(),
                        allow_overwrite,
                    })
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            anyhow::ensure!(
                !diffs.is_empty() || !new_files.is_empty(),
                "No file changes supplied"
            );
            Tool::ApplyFileDiffs(ApplyFileDiffs {
                summary,
                diffs,
                new_files,
                ..Default::default()
            })
        }
        _ => anyhow::bail!("Unknown tool requested by provider"),
    };
    Ok(MessageKind::ToolCall(ToolCall {
        tool_call_id: tool_id,
        tool: Some(tool),
    }))
}

fn responses_tool_call(tc: &ToolCall) -> anyhow::Result<Value> {
    let (name, arguments) = match &tc.tool {
        Some(Tool::RunShellCommand(cmd)) => ("run_shell_command", json!({"command": cmd.command})),
        Some(Tool::ReadFiles(files)) => (
            "read_files",
            json!({"file_paths": files.files.iter().map(|file| &file.name).collect::<Vec<_>>()}),
        ),
        Some(Tool::ApplyFileDiffs(diffs)) => (
            "apply_file_diffs",
            json!({
                "summary": diffs.summary,
                "diffs": diffs.diffs.iter().map(|diff| json!({"file_path": diff.file_path, "search": diff.search, "replace": diff.replace})).collect::<Vec<_>>(),
                "new_files": diffs.new_files.iter().map(|file| json!({"file_path": file.file_path, "content": file.content, "allow_overwrite": file.allow_overwrite})).collect::<Vec<_>>()
            }),
        ),
        _ => anyhow::bail!("Conversation contains a tool unsupported by this endpoint"),
    };
    Ok(
        json!({"type": "function_call", "call_id": tc.tool_call_id, "name": name, "arguments": arguments.to_string()}),
    )
}

fn build_responses_payload(
    model: &str,
    tasks: &[api::Task],
    current_input: &api::request::Input,
) -> anyhow::Result<Value> {
    let mut input = Vec::new();
    for task in tasks {
        for message in &task.messages {
            if let Some(MessageKind::ToolCall(tc)) = &message.message {
                input.push(responses_tool_call(tc)?);
                continue;
            }
            let single_task = api::Task {
                messages: vec![message.clone()],
                ..Default::default()
            };
            for (role, content) in
                extract_messages_for_llm(&[single_task], &api::request::Input::default())
            {
                if let Some(MessageKind::ToolCallResult(result)) = &message.message {
                    input.push(json!({"type": "function_call_output", "call_id": result.tool_call_id, "output": content}));
                } else {
                    input.push(json!({"role": role, "content": content}));
                }
            }
        }
    }
    if let Some(api::request::input::Type::UserInputs(inputs)) = &current_input.r#type {
        for item in &inputs.inputs {
            let single_input = api::request::Input {
                r#type: Some(api::request::input::Type::UserInputs(
                    api::request::input::UserInputs {
                        inputs: vec![item.clone()],
                    },
                )),
                ..Default::default()
            };
            for (role, content) in extract_messages_for_llm(&[], &single_input) {
                if let Some(api::request::input::user_inputs::user_input::Input::ToolCallResult(
                    result,
                )) = &item.input
                {
                    input.push(json!({"type": "function_call_output", "call_id": result.tool_call_id, "output": content}));
                } else {
                    input.push(json!({"role": role, "content": content}));
                }
            }
        }
    } else {
        input.extend(
            extract_messages_for_llm(&[], current_input)
                .into_iter()
                .map(|(role, content)| json!({"role": role, "content": content})),
        );
    }
    let tools = get_openai_tools()
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| {
            let mut function = tool["function"].clone();
            function["type"] = json!("function");
            function["strict"] = json!(false);
            function
        })
        .collect::<Vec<_>>();
    Ok(
        json!({"model": model, "instructions": format_system_prompt_with_context(current_input.context.as_ref()), "input": input, "tools": tools, "stream": true, "store": false}),
    )
}

fn inference_client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder().redirect(reqwest::redirect::Policy::none())
}

#[allow(clippy::too_many_arguments)]
async fn run_openai_stream(
    api_key: &str,
    base_url: &str,
    model: &str,
    is_responses_endpoint: bool,
    task_id: &str,
    request_id: &str,
    tasks: &[api::Task],
    current_input: &api::request::Input,
    tx: &async_channel::Sender<Result<ResponseEvent, Arc<AIApiError>>>,
) -> anyhow::Result<()> {
    let client = inference_client_builder().build()?;
    let url = if is_responses_endpoint {
        if base_url.ends_with("/responses") {
            base_url.to_string()
        } else {
            format!("{}/responses", base_url.trim_end_matches('/'))
        }
    } else if base_url.ends_with("/chat/completions") {
        base_url.to_string()
    } else {
        format!("{}/chat/completions", base_url.trim_end_matches('/'))
    };

    let system_prompt = format_system_prompt_with_context(current_input.context.as_ref());
    let mut api_messages = vec![json!({
        "role": "system",
        "content": system_prompt
    })];

    for (role, content) in extract_messages_for_llm(tasks, current_input) {
        api_messages.push(json!({
            "role": role,
            "content": content
        }));
    }

    let payload = if is_responses_endpoint {
        build_responses_payload(model, tasks, current_input)?
    } else {
        json!({"model": model, "messages": api_messages, "tools": get_openai_tools(), "stream": true})
    };

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .header("User-Agent", "Unwrap")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("OpenAI API error ({status}): {body}");
    }

    consume_stream(
        response.bytes_stream(),
        if is_responses_endpoint {
            StreamProtocol::Responses
        } else {
            StreamProtocol::Chat
        },
        task_id,
        request_id,
        tx,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_anthropic_stream(
    api_key: &str,
    base_url: Option<&str>,
    model: &str,
    task_id: &str,
    request_id: &str,
    tasks: &[api::Task],
    current_input: &api::request::Input,
    tx: &async_channel::Sender<Result<ResponseEvent, Arc<AIApiError>>>,
) -> anyhow::Result<()> {
    let client = inference_client_builder().build()?;
    let url = if let Some(base) = base_url {
        let trimmed = base.trim_end_matches('/');
        if trimmed.ends_with("/messages") {
            trimmed.to_string()
        } else {
            format!("{trimmed}/messages")
        }
    } else {
        "https://api.anthropic.com/v1/messages".to_string()
    };

    let mut api_messages = Vec::new();
    for (role, content) in extract_messages_for_llm(tasks, current_input) {
        api_messages.push(json!({
            "role": role,
            "content": content
        }));
    }

    if api_messages.is_empty() {
        api_messages.push(json!({
            "role": "user",
            "content": "Hello"
        }));
    }

    let system_prompt = format_system_prompt_with_context(current_input.context.as_ref());

    let payload = json!({
        "model": model,
        "max_tokens": 4096,
        "system": system_prompt,
        "messages": api_messages,
        "tools": get_anthropic_tools(),
        "stream": true
    });

    let response = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .header("User-Agent", "Unwrap")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Anthropic API error ({status}): {body}");
    }

    consume_stream(
        response.bytes_stream(),
        StreamProtocol::Anthropic,
        task_id,
        request_id,
        tx,
    )
    .await
}

#[cfg(test)]
#[path = "local_byok_tests.rs"]
mod tests;
