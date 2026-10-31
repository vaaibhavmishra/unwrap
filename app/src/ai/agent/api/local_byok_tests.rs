use warp_multi_agent_api::request::settings::custom_model_providers::{
    CustomEndpointSchema, CustomModel, CustomModelProvider,
};
use warp_multi_agent_api::request::settings::{ApiKeys, CustomModelProviders};

use super::*;

fn test_request_params(model_str: &str) -> RequestParams {
    RequestParams {
        model: model_str.into(),
        ..RequestParams::new_for_test()
    }
}

#[test]
fn resolve_provider_selects_custom_model_by_config_key() {
    let mut params = test_request_params("custom-uuid-123");
    params.api_keys = Some(ApiKeys {
        anthropic: "sk-ant-test".to_string(),
        openai: "sk-proj-test".to_string(),
        ..Default::default()
    });
    params.custom_model_providers = Some(CustomModelProviders {
        providers: vec![
            CustomModelProvider {
                base_url: "https://first-provider.example.com".to_string(),
                api_key: "first-key".to_string(),
                schema: CustomEndpointSchema::OpenaiChatCompletions as i32,
                models: vec![CustomModel {
                    slug: "first-model".to_string(),
                    config_key: "first-uuid".to_string(),
                    reasoning_effort: String::new(),
                }],
            },
            CustomModelProvider {
                base_url: "https://custom.example.com/v1".to_string(),
                api_key: "custom-key".to_string(),
                schema: CustomEndpointSchema::AnthropicMessages as i32,
                models: vec![CustomModel {
                    slug: "my-custom-model".to_string(),
                    config_key: "custom-uuid-123".to_string(),
                    reasoning_effort: String::new(),
                }],
            },
        ],
    });

    let resolved = resolve_provider(&params)
        .expect("valid selection")
        .expect("should resolve custom provider");
    match resolved {
        ByokProvider::Anthropic {
            api_key,
            base_url,
            model,
        } => {
            assert_eq!(api_key, "custom-key");
            assert_eq!(base_url, Some("https://custom.example.com/v1".to_string()));
            assert_eq!(model, "my-custom-model");
        }
        _ => panic!("expected Anthropic provider"),
    }
}

#[test]
fn resolve_provider_routes_to_requested_standard_model_when_custom_provider_present() {
    let mut params = test_request_params("claude-3-7-sonnet");
    params.api_keys = Some(ApiKeys {
        anthropic: "sk-ant-real-key".to_string(),
        openai: "sk-openai-key".to_string(),
        ..Default::default()
    });
    params.custom_model_providers = Some(CustomModelProviders {
        providers: vec![CustomModelProvider {
            base_url: "https://ollama.local:11434".to_string(),
            api_key: "ollama-dummy".to_string(),
            schema: CustomEndpointSchema::OpenaiChatCompletions as i32,
            models: vec![CustomModel {
                slug: "llama3".to_string(),
                config_key: "llama3-uuid".to_string(),
                reasoning_effort: String::new(),
            }],
        }],
    });

    let resolved = resolve_provider(&params)
        .expect("valid selection")
        .expect("should resolve Anthropic provider");
    match resolved {
        ByokProvider::Anthropic {
            api_key,
            base_url,
            model,
        } => {
            assert_eq!(api_key, "sk-ant-real-key");
            assert_eq!(base_url, None);
            assert_eq!(model, "claude-3-7-sonnet");
        }
        _ => panic!("expected Anthropic provider"),
    }
}

#[test]
fn apply_file_diffs_tool_parser_defaults_allow_overwrite_to_false() {
    let args = json!({
        "summary": "Create new file",
        "diffs": [],
        "new_files": [
            {
                "file_path": "src/main.rs",
                "content": "fn main() {}"
            }
        ]
    })
    .to_string();

    let tool = parse_tool_call_args("call-1".to_string(), "apply_file_diffs", &args)
        .expect("should parse");

    match tool {
        MessageKind::ToolCall(tc) => match tc.tool {
            Some(Tool::ApplyFileDiffs(afd)) => {
                assert_eq!(afd.new_files.len(), 1);
                assert!(
                    !afd.new_files[0].allow_overwrite,
                    "allow_overwrite must be false by default"
                );
            }
            other => panic!("expected ApplyFileDiffs, got {other:?}"),
        },
        other => panic!("expected ToolCall, got {other:?}"),
    }
}

#[test]
fn format_apply_file_diffs_result_preserves_error_message() {
    let err_result = api::ApplyFileDiffsResult {
        result: Some(api::apply_file_diffs_result::Result::Error(
            api::apply_file_diffs_result::Error {
                message: "Diff search chunk not found in target file".to_string(),
            },
        )),
    };
    let formatted = format_apply_file_diffs_result(&err_result);
    assert!(formatted.contains("Error applying file diffs"));
    assert!(formatted.contains("Diff search chunk not found in target file"));
}

#[test]
fn explicit_missing_model_never_falls_back_to_cloud() {
    let mut params = test_request_params("removed-local-model");
    params.api_keys = Some(ApiKeys {
        anthropic: "test-key".into(),
        ..Default::default()
    });
    assert!(resolve_provider(&params).is_err());
}

fn custom_endpoint_params(model: &str) -> RequestParams {
    let mut params = test_request_params(model);
    params.custom_model_providers = Some(CustomModelProviders {
        providers: vec![CustomModelProvider {
            base_url: "http://localhost:11434/v1".into(),
            api_key: "local".into(),
            schema: CustomEndpointSchema::OpenaiResponses as i32,
            models: vec![CustomModel {
                config_key: "custom-id".into(),
                slug: "claude-local".into(),
                ..Default::default()
            }],
        }],
    });
    params
}

#[test]
fn explicit_provider_without_key_never_uses_custom_endpoint() {
    assert!(resolve_provider(&custom_endpoint_params("claude-local")).is_err());
}

#[test]
fn custom_slug_cannot_shadow_builtin_selection() {
    let mut params = custom_endpoint_params("claude-local");
    params.api_keys = Some(ApiKeys {
        anthropic: "anthropic-key".into(),
        ..Default::default()
    });
    assert!(matches!(
        resolve_provider(&params).unwrap(),
        Some(ByokProvider::Anthropic { base_url: None, .. })
    ));
}

#[test]
fn only_auto_allows_provider_fallback() {
    assert!(matches!(
        resolve_provider(&custom_endpoint_params("auto")).unwrap(),
        Some(ByokProvider::OpenAiResponses { .. })
    ));
    assert!(resolve_provider(&custom_endpoint_params("")).is_err());
    assert!(
        resolve_provider(&test_request_params("auto"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn openrouter_selection_uses_its_key_and_unqualified_model_id() {
    let mut params = test_request_params("openrouter/anthropic/claude-sonnet-4.5");
    params.api_keys = Some(ApiKeys {
        anthropic: "wrong-key".into(),
        open_router: "router-key".into(),
        ..Default::default()
    });
    match resolve_provider(&params).unwrap().unwrap() {
        ByokProvider::OpenAiChatCompletions {
            api_key,
            base_url,
            model,
        } => {
            assert_eq!(api_key, "router-key");
            assert_eq!(base_url, "https://openrouter.ai/api/v1");
            assert_eq!(model, "anthropic/claude-sonnet-4.5");
        }
        _ => panic!("expected OpenRouter"),
    }
}

#[test]
fn explicit_gemini_model_is_preserved() {
    let mut params = test_request_params("gemini-2.5-flash");
    params.api_keys = Some(ApiKeys {
        google: "google-key".into(),
        ..Default::default()
    });
    assert!(
        matches!(resolve_provider(&params).unwrap(), Some(ByokProvider::OpenAiChatCompletions { model, .. }) if model == "gemini-2.5-flash")
    );
}

#[test]
fn local_catalog_models_resolve_with_configured_keys() {
    for (model, _, _) in crate::ai::local_models::LOCAL_MODELS {
        let mut params = test_request_params(model);
        params.api_keys = Some(ApiKeys {
            anthropic: "anthropic".into(),
            openai: "openai".into(),
            google: "google".into(),
            open_router: "router".into(),
            ..Default::default()
        });
        assert!(resolve_provider(&params).unwrap().is_some(), "{model}");
    }
}

#[test]
fn invalid_tool_arguments_are_rejected() {
    assert!(parse_tool_call_args("id".into(), "run_shell_command", "{").is_err());
    assert!(parse_tool_call_args("id".into(), "run_shell_command", "{}").is_err());
    assert!(
        parse_tool_call_args("id".into(), "read_files", r#"{"file_paths":["valid",42]}"#).is_err()
    );
    assert!(
        parse_tool_call_args(
            "id".into(),
            "apply_file_diffs",
            r#"{"diffs":[{"file_path":"a","search":"a","replace":"b"},{}]}"#
        )
        .is_err()
    );
    assert!(
        parse_tool_call_args(
            "id".into(),
            "apply_file_diffs",
            r#"{"new_files":[{"file_path":"a","content":"b","allow_overwrite":"false"}]}"#
        )
        .is_err()
    );
    assert!(parse_tool_call_args("id".into(), "unknown", "{}").is_err());
}

fn sse(events: &[Value]) -> String {
    events
        .iter()
        .map(|event| format!("data: {event}\n\n"))
        .collect()
}

async fn run_test_stream(
    protocol: StreamProtocol,
    data: &str,
) -> (anyhow::Result<()>, Vec<MessageKind>) {
    let chunks = data
        .as_bytes()
        .chunks(1)
        .map(|bytes| Ok::<_, anyhow::Error>(bytes.to_vec()))
        .collect::<Vec<_>>();
    let (tx, rx) = async_channel::unbounded();
    let result = consume_stream(
        futures_util::stream::iter(chunks),
        protocol,
        "task",
        "request",
        &tx,
    )
    .await;
    let mut messages = Vec::new();
    while let Ok(event) = rx.try_recv() {
        if let Some(response_event::Type::ClientActions(actions)) = event.unwrap().r#type {
            for action in actions.actions {
                match action.action {
                    Some(Action::AddMessagesToTask(action)) => messages.extend(
                        action
                            .messages
                            .into_iter()
                            .filter_map(|message| message.message),
                    ),
                    Some(Action::AppendToMessageContent(action)) => {
                        messages.extend(action.message.and_then(|message| message.message))
                    }
                    _ => {}
                }
            }
        }
    }
    (result, messages)
}

#[tokio::test]
async fn chat_handles_split_utf8_crlf_and_data_without_space() {
    let (result, messages) = run_test_stream(StreamProtocol::Chat,
        ": ping\r\nevent: message\r\ndata:{\"choices\":[{\"delta\":{\"content\":\"Hello 🌍\"},\"finish_reason\":null}]}\r\n\r\ndata: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata:[DONE]\n\n").await;
    result.unwrap();
    assert!(
        matches!(&messages[..], [MessageKind::AgentOutput(output)] if output.text == "Hello 🌍")
    );
}

#[tokio::test]
async fn truncated_text_is_an_error_in_each_protocol() {
    let (chat, _) = run_test_stream(
        StreamProtocol::Chat,
        &sse(&[json!({"choices":[{"delta":{"content":"Partial"}}]})]),
    )
    .await;
    let (anthropic, _) = run_test_stream(
        StreamProtocol::Anthropic,
        &sse(&[
            json!({"type":"content_block_delta","delta":{"type":"text_delta","text":"Partial"}}),
        ]),
    )
    .await;
    let (responses, _) = run_test_stream(
        StreamProtocol::Responses,
        &sse(&[json!({"type":"response.output_text.delta","delta":"Partial"})]),
    )
    .await;
    assert!(chat.is_err());
    assert!(anthropic.is_err());
    assert!(responses.is_err());
}

#[tokio::test]
async fn unterminated_tool_call_never_reaches_execution() {
    let (result, messages) = run_test_stream(StreamProtocol::Chat, &sse(&[json!({"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call","function":{"name":"run_shell_command","arguments":"{\"command\":\"pwd\"}"}}]}}]})])).await;
    assert!(result.is_err());
    assert!(messages.is_empty());
}

#[tokio::test]
async fn chat_requires_both_successful_finish_reason_and_done() {
    let (missing_reason, _) = run_test_stream(StreamProtocol::Chat, "data: [DONE]\n\n").await;
    let (missing_done, _) = run_test_stream(
        StreamProtocol::Chat,
        &sse(&[json!({"choices":[{"delta":{},"finish_reason":"stop"}]})]),
    )
    .await;
    let (length, _) = run_test_stream(
        StreamProtocol::Chat,
        &sse(&[json!({"choices":[{"delta":{},"finish_reason":"length"}]})]),
    )
    .await;
    assert!(missing_reason.is_err());
    assert!(missing_done.is_err());
    assert!(length.is_err());
}

#[tokio::test]
async fn malformed_event_cannot_be_skipped_to_execute_tools() {
    let (result, messages) = run_test_stream(
        StreamProtocol::Chat,
        "data: {broken json}\n\ndata: [DONE]\n\n",
    )
    .await;
    assert!(result.is_err());
    assert!(messages.is_empty());
}

#[tokio::test]
async fn malformed_member_rejects_entire_tool_batch() {
    let mut data = sse(&[json!({"choices":[{"delta":{"tool_calls":[
        {"index":0,"id":"valid","function":{"name":"run_shell_command","arguments":"{\"command\":\"pwd\"}"}},
        {"index":1,"id":"invalid","function":{"name":"read_files","arguments":"{\"file_paths\":[42]}"}}
    ]},"finish_reason":"tool_calls"}]})]);
    data.push_str("data: [DONE]\n\n");
    let (result, messages) = run_test_stream(StreamProtocol::Chat, &data).await;
    assert!(result.is_err());
    assert!(messages.is_empty());
}

#[tokio::test]
async fn complete_chat_tool_batch_is_emitted() {
    let mut data = sse(&[
        json!({"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call","function":{"name":"run_shell_command","arguments":"{\"command\":\"pwd\"}"}}]},"finish_reason":"tool_calls"}]}),
    ]);
    data.push_str("data: [DONE]\n\n");
    let (result, messages) = run_test_stream(StreamProtocol::Chat, &data).await;
    result.unwrap();
    assert!(matches!(&messages[..], [MessageKind::ToolCall(call)] if call.tool_call_id == "call"));
}

#[tokio::test]
async fn anthropic_completes_only_after_successful_stop() {
    let data = sse(&[
        json!({"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"call","name":"run_shell_command","input":{}}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"command\":\"pwd\"}"}}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"message_delta","delta":{"stop_reason":"tool_use"}}),
        json!({"type":"message_stop"}),
    ]);
    let (result, messages) = run_test_stream(StreamProtocol::Anthropic, &data).await;
    result.unwrap();
    assert!(matches!(&messages[..], [MessageKind::ToolCall(call)] if call.tool_call_id == "call"));
    let (truncated, messages) = run_test_stream(
        StreamProtocol::Anthropic,
        &data.replace("tool_use\"}", "max_tokens\"}"),
    )
    .await;
    assert!(truncated.is_err());
    assert!(messages.is_empty());
}

#[tokio::test]
async fn responses_stream_emits_text_and_completed_function_call() {
    let item = json!({"type":"function_call","id":"fc_1","call_id":"call_1","name":"run_shell_command","arguments":"{\"command\":\"pwd\"}","status":"completed"});
    let data = sse(&[
        json!({"type":"response.output_text.delta","delta":"Checking"}),
        json!({"type":"response.output_item.added","output_index":1,"item":{"type":"function_call","id":"fc_1","call_id":"call_1","name":"run_shell_command","arguments":""}}),
        json!({"type":"response.function_call_arguments.delta","output_index":1,"item_id":"fc_1","delta":"{\"command\":"}),
        json!({"type":"response.function_call_arguments.delta","output_index":1,"item_id":"fc_1","delta":"\"pwd\"}"}),
        json!({"type":"response.function_call_arguments.done","output_index":1,"item_id":"fc_1","arguments":"{\"command\":\"pwd\"}"}),
        json!({"type":"response.output_item.done","output_index":1,"item":item}),
        json!({"type":"response.completed","response":{"status":"completed","output":[{"type":"message","status":"completed"},item]}}),
    ]);
    let (result, messages) = run_test_stream(StreamProtocol::Responses, &data).await;
    result.unwrap();
    assert!(
        matches!(&messages[..], [MessageKind::AgentOutput(text), MessageKind::ToolCall(call)] if text.text == "Checking" && call.tool_call_id == "call_1")
    );
}

#[tokio::test]
async fn responses_incomplete_and_failed_are_errors() {
    let (incomplete, messages) = run_test_stream(
        StreamProtocol::Responses,
        &sse(&[json!({"type":"response.incomplete","response":{"status":"incomplete"}})]),
    )
    .await;
    assert!(incomplete.is_err());
    assert!(messages.is_empty());
    let (failed, _) = run_test_stream(
        StreamProtocol::Responses,
        &sse(&[json!({"type":"response.failed","response":{"status":"failed"}})]),
    )
    .await;
    assert!(failed.is_err());
}

#[test]
fn responses_payload_preserves_tool_calls_and_outputs() {
    let task = api::Task {
        messages: vec![Message {
            message: Some(MessageKind::ToolCall(ToolCall {
                tool_call_id: "call_1".into(),
                tool: Some(Tool::RunShellCommand(RunShellCommand {
                    command: "pwd".into(),
                    ..Default::default()
                })),
            })),
            ..Default::default()
        }],
        ..Default::default()
    };
    let input = api::request::Input {
        r#type: Some(api::request::input::Type::UserInputs(
            api::request::input::UserInputs {
                inputs: vec![api::request::input::user_inputs::UserInput {
                    input: Some(
                        api::request::input::user_inputs::user_input::Input::ToolCallResult(
                            api::request::input::ToolCallResult {
                                tool_call_id: "call_1".into(),
                                ..Default::default()
                            },
                        ),
                    ),
                }],
            },
        )),
        ..Default::default()
    };
    let payload = build_responses_payload("test", &[task], &input).unwrap();
    assert!(payload.get("messages").is_none());
    assert_eq!(payload["store"], false);
    assert_eq!(payload["instructions"], SYSTEM_PROMPT);
    assert_eq!(payload["tools"][0]["type"], "function");
    assert_eq!(payload["tools"][0]["name"], "run_shell_command");
    assert_eq!(payload["tools"][0]["strict"], false);
    assert_eq!(
        payload["input"][0],
        json!({"type":"function_call","call_id":"call_1","name":"run_shell_command","arguments":"{\"command\":\"pwd\"}"})
    );
    assert_eq!(payload["input"][1]["type"], "function_call_output");
    assert_eq!(payload["input"][1]["call_id"], "call_1");
}

#[tokio::test]
async fn inference_redirect_does_not_forward_credentials_or_body() {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let target = TcpListener::bind("127.0.0.1:0").unwrap();
    target.set_nonblocking(true).unwrap();
    let source = TcpListener::bind("127.0.0.1:0").unwrap();
    let source_url = format!("http://{}", source.local_addr().unwrap());
    let target_url = format!("http://{}", target.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut socket, _) = source.accept().unwrap();
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut buffer = [0; 4096];
        let mut received = Vec::new();
        loop {
            let size = socket.read(&mut buffer).unwrap();
            assert!(size > 0, "request ended before its body");
            received.extend_from_slice(&buffer[..size]);
            if received.ends_with(b"private prompt") {
                break;
            }
        }
        write!(socket, "HTTP/1.1 307 Temporary Redirect\r\nLocation: {target_url}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
    });
    let response = inference_client_builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap()
        .post(source_url)
        .header("x-api-key", "dummy-key")
        .bearer_auth("dummy-token")
        .body("private prompt")
        .send()
        .await
        .unwrap();
    server.join().unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        target.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn custom_endpoint_url_is_never_rewritten_by_hostname_substrings() {
    let mut params = custom_endpoint_params("custom-id");
    params.custom_model_providers.as_mut().unwrap().providers[0].base_url =
        "https://proxy.example/generativelanguage.googleapis.com/v1/".into();
    assert!(
        matches!(resolve_provider(&params).unwrap(), Some(ByokProvider::OpenAiResponses { base_url, .. }) if base_url == "https://proxy.example/generativelanguage.googleapis.com/v1")
    );
}

#[tokio::test]
async fn chat_accepts_multiline_sse_data() {
    let (result, _) = run_test_stream(StreamProtocol::Chat, "data: {\"choices\": [\ndata: {\"delta\": {}, \"finish_reason\": \"stop\"}]}\n\ndata: [DONE]\n\n").await;
    result.unwrap();
}

#[tokio::test]
async fn transport_error_never_emits_pending_tools() {
    let data = sse(&[
        json!({"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call","function":{"name":"run_shell_command","arguments":"{\"command\":\"pwd\"}"}}]}}]}),
    ]);
    let (tx, rx) = async_channel::unbounded();
    let chunks = futures_util::stream::iter(vec![
        Ok(data.into_bytes()),
        Err(anyhow::anyhow!("connection reset")),
    ]);
    assert!(
        consume_stream(chunks, StreamProtocol::Chat, "task", "request", &tx)
            .await
            .is_err()
    );
    assert!(rx.is_empty());
}
