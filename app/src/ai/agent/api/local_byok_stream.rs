use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::time::SystemTime;

use futures_util::{Stream, StreamExt};
use prost_types::FieldMask;
use serde_json::Value;
use uuid::Uuid;
use warp_multi_agent_api::client_action::{Action, AddMessagesToTask, AppendToMessageContent};
use warp_multi_agent_api::message::{AgentOutput, Message as MessageKind};
use warp_multi_agent_api::{ClientAction, Message, ResponseEvent, response_event};

use super::{parse_tool_call_args, required_string};
use crate::server::server_api::AIApiError;

const MAX_EVENT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy)]
pub(super) enum StreamProtocol {
    Chat,
    Responses,
    Anthropic,
}

#[derive(Default)]
struct SseDecoder {
    line: Vec<u8>,
    data: String,
}

impl SseDecoder {
    fn push(&mut self, byte: u8) -> anyhow::Result<Option<String>> {
        if byte != b'\n' {
            anyhow::ensure!(
                self.line.len() + self.data.len() < MAX_EVENT_BYTES,
                "Provider event is too large"
            );
            self.line.push(byte);
            return Ok(None);
        }
        if self.line.last() == Some(&b'\r') {
            self.line.pop();
        }
        let line = std::str::from_utf8(&self.line)?;
        if line.is_empty() {
            if self.data.is_empty() {
                return Ok(None);
            }
            self.data.pop();
            return Ok(Some(std::mem::take(&mut self.data)));
        }
        if let Some(data) = line.strip_prefix("data:") {
            self.data.push_str(data.strip_prefix(' ').unwrap_or(data));
            self.data.push('\n');
        }
        self.line.clear();
        Ok(None)
    }
}

#[derive(Default)]
struct StreamedToolCall {
    id: String,
    name: String,
    args: String,
}

#[derive(Default)]
struct StreamState {
    tools: BTreeMap<u64, StreamedToolCall>,
    stop_reason: Option<String>,
    completed: bool,
}

impl StreamState {
    fn event(&mut self, protocol: StreamProtocol, data: &str) -> anyhow::Result<Option<String>> {
        if matches!(protocol, StreamProtocol::Chat) && data == "[DONE]" {
            self.validate_stop("stop", "tool_calls")?;
            self.completed = true;
            return Ok(None);
        }
        let value: Value = serde_json::from_str(data)?;
        anyhow::ensure!(
            value.get("error").is_none_or(Value::is_null),
            "Provider reported a streaming error"
        );
        let text = match protocol {
            StreamProtocol::Chat => self.chat(&value)?,
            StreamProtocol::Responses => self.responses(&value)?,
            StreamProtocol::Anthropic => self.anthropic(&value)?,
        };
        anyhow::ensure!(
            self.tools.len() <= 1024
                && self
                    .tools
                    .values()
                    .map(|tool| tool.args.len())
                    .sum::<usize>()
                    <= MAX_EVENT_BYTES,
            "Provider tool batch is too large"
        );
        Ok(text)
    }

    fn validate_stop(&self, text_stop: &str, tool_stop: &str) -> anyhow::Result<()> {
        let reason = self.stop_reason.as_deref();
        let valid = if self.tools.is_empty() {
            reason == Some(text_stop)
                || (text_stop == "end_turn" && reason == Some("stop_sequence"))
        } else {
            reason == Some(tool_stop)
        };
        anyhow::ensure!(valid, "Provider stream did not finish successfully");
        Ok(())
    }

    fn chat(&mut self, value: &Value) -> anyhow::Result<Option<String>> {
        let choices = value
            .get("choices")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("Missing chat choices"))?;
        let Some(choice) = choices.first() else {
            return Ok(None);
        };
        anyhow::ensure!(
            self.stop_reason.is_none(),
            "Received output after chat completion"
        );
        let delta = &choice["delta"];
        anyhow::ensure!(
            delta.get("refusal").is_none_or(Value::is_null),
            "Provider refused the request"
        );
        if let Some(calls) = delta.get("tool_calls").and_then(Value::as_array) {
            for call in calls {
                let index = index(call, "index")?;
                let tool = self.tools.entry(index).or_default();
                if let Some(id) = call.get("id").and_then(Value::as_str) {
                    tool.id.push_str(id);
                }
                if let Some(function) = call.get("function") {
                    if let Some(name) = function.get("name").and_then(Value::as_str) {
                        tool.name.push_str(name);
                    }
                    if let Some(args) = function.get("arguments").and_then(Value::as_str) {
                        tool.args.push_str(args);
                    }
                }
            }
        }
        if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
            anyhow::ensure!(
                matches!(reason, "stop" | "tool_calls"),
                "Provider stopped before completing the response ({reason})"
            );
            self.stop_reason = Some(reason.to_string());
        }
        Ok(delta
            .get("content")
            .and_then(Value::as_str)
            .map(str::to_string))
    }

    fn anthropic(&mut self, value: &Value) -> anyhow::Result<Option<String>> {
        match required_string(value, "type")? {
            "error" => anyhow::bail!("Anthropic reported a streaming error"),
            "message_stop" => {
                self.validate_stop("end_turn", "tool_use")?;
                self.completed = true;
            }
            "message_delta" => {
                if let Some(reason) = value["delta"].get("stop_reason").and_then(Value::as_str) {
                    anyhow::ensure!(
                        matches!(reason, "end_turn" | "tool_use" | "stop_sequence"),
                        "Anthropic stopped before completing the response ({reason})"
                    );
                    self.stop_reason = Some(reason.to_string());
                }
            }
            "content_block_start" => {
                anyhow::ensure!(
                    self.stop_reason.is_none(),
                    "Received output after Anthropic completion"
                );
                let block = &value["content_block"];
                if block["type"] == "tool_use" {
                    let index = index(value, "index")?;
                    anyhow::ensure!(!self.tools.contains_key(&index), "Duplicate tool block");
                    self.tools.insert(
                        index,
                        StreamedToolCall {
                            id: required_string(block, "id")?.to_string(),
                            name: required_string(block, "name")?.to_string(),
                            args: match block.get("input").and_then(Value::as_object) {
                                Some(input) if !input.is_empty() => {
                                    Value::Object(input.clone()).to_string()
                                }
                                _ => String::new(),
                            },
                        },
                    );
                } else if block["type"] == "text" {
                    return Ok(block
                        .get("text")
                        .and_then(Value::as_str)
                        .map(str::to_string));
                }
            }
            "content_block_delta" => {
                anyhow::ensure!(
                    self.stop_reason.is_none(),
                    "Received output after Anthropic completion"
                );
                let delta = &value["delta"];
                match required_string(delta, "type")? {
                    "text_delta" => return Ok(Some(required_string(delta, "text")?.to_string())),
                    "input_json_delta" => {
                        let tool = self
                            .tools
                            .get_mut(&index(value, "index")?)
                            .ok_or_else(|| anyhow::anyhow!("Missing tool block"))?;
                        tool.args.push_str(required_string(delta, "partial_json")?);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn responses(&mut self, value: &Value) -> anyhow::Result<Option<String>> {
        match required_string(value, "type")? {
            "response.failed"
            | "response.incomplete"
            | "error"
            | "response.refusal.delta"
            | "response.refusal.done" => {
                anyhow::bail!("Responses API did not complete the request")
            }
            "response.output_text.delta" => {
                return Ok(Some(required_string(value, "delta")?.to_string()));
            }
            "response.output_item.added" | "response.output_item.done" => {
                let item = &value["item"];
                if item["type"] == "function_call" {
                    self.tools
                        .insert(index(value, "output_index")?, response_tool(item)?);
                }
            }
            "response.function_call_arguments.delta" => {
                let tool = self
                    .tools
                    .get_mut(&index(value, "output_index")?)
                    .ok_or_else(|| anyhow::anyhow!("Missing Responses function call"))?;
                tool.args.push_str(required_string(value, "delta")?);
            }
            "response.function_call_arguments.done" => {
                let tool = self
                    .tools
                    .get_mut(&index(value, "output_index")?)
                    .ok_or_else(|| anyhow::anyhow!("Missing Responses function call"))?;
                tool.args = required_string(value, "arguments")?.to_string();
            }
            "response.completed" => {
                let response = &value["response"];
                anyhow::ensure!(
                    response["status"] == "completed"
                        && response.get("error").is_none_or(Value::is_null)
                        && response
                            .get("incomplete_details")
                            .is_none_or(Value::is_null),
                    "Responses API did not complete successfully"
                );
                let output = response
                    .get("output")
                    .and_then(Value::as_array)
                    .ok_or_else(|| anyhow::anyhow!("Missing Responses output"))?;
                let mut completed_tools = BTreeMap::new();
                for (index, item) in output.iter().enumerate() {
                    anyhow::ensure!(
                        item.get("status")
                            .is_none_or(|status| status == "completed"),
                        "Incomplete Responses output item"
                    );
                    if item["type"] == "function_call" {
                        completed_tools.insert(index as u64, response_tool(item)?);
                    }
                }
                for tool in self.tools.values() {
                    anyhow::ensure!(
                        completed_tools
                            .values()
                            .any(|completed| completed.id == tool.id
                                && completed.name == tool.name
                                && completed.args == tool.args),
                        "Incomplete Responses function call"
                    );
                }
                self.tools = completed_tools;
                self.completed = true;
            }
            _ => {}
        }
        Ok(None)
    }

    fn tool_messages(self, task_id: &str, request_id: &str) -> anyhow::Result<Vec<Message>> {
        anyhow::ensure!(
            self.completed,
            "Provider stream terminated prematurely without successful completion"
        );
        let mut ids = HashSet::new();
        self.tools
            .into_values()
            .map(|tool| {
                anyhow::ensure!(ids.insert(tool.id.clone()), "Duplicate tool call ID");
                let kind = parse_tool_call_args(tool.id, &tool.name, &tool.args)?;
                Ok(message(
                    kind,
                    Uuid::new_v4().to_string(),
                    task_id,
                    request_id,
                ))
            })
            .collect()
    }
}

fn index(value: &Value, field: &str) -> anyhow::Result<u64> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid stream index"))
}

fn response_tool(item: &Value) -> anyhow::Result<StreamedToolCall> {
    Ok(StreamedToolCall {
        id: required_string(item, "call_id")?.to_string(),
        name: required_string(item, "name")?.to_string(),
        args: required_string(item, "arguments")?.to_string(),
    })
}

fn message(kind: MessageKind, id: String, task_id: &str, request_id: &str) -> Message {
    Message {
        id,
        task_id: task_id.to_string(),
        request_id: request_id.to_string(),
        timestamp: Some(prost_types::Timestamp::from(SystemTime::now())),
        message: Some(kind),
        ..Default::default()
    }
}

async fn send_action(
    tx: &async_channel::Sender<Result<ResponseEvent, Arc<AIApiError>>>,
    action: Action,
) -> anyhow::Result<()> {
    tx.send(Ok(ResponseEvent {
        r#type: Some(response_event::Type::ClientActions(
            response_event::ClientActions {
                actions: vec![ClientAction {
                    action: Some(action),
                }],
            },
        )),
    }))
    .await
    .map_err(|_| anyhow::anyhow!("Response consumer closed"))
}

pub(super) async fn consume_stream<S, B, E>(
    stream: S,
    protocol: StreamProtocol,
    task_id: &str,
    request_id: &str,
    tx: &async_channel::Sender<Result<ResponseEvent, Arc<AIApiError>>>,
) -> anyhow::Result<()>
where
    S: Stream<Item = Result<B, E>>,
    B: AsRef<[u8]>,
    E: Into<anyhow::Error>,
{
    futures_util::pin_mut!(stream);
    let mut decoder = SseDecoder::default();
    let mut state = StreamState::default();
    let mut open_message_id: Option<String> = None;
    'stream: while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(Into::into)?;
        for byte in chunk.as_ref() {
            if let Some(data) = decoder.push(*byte)? {
                if let Some(text) = state.event(protocol, &data)?
                    && !text.is_empty()
                {
                    let action = if let Some(id) = &open_message_id {
                        Action::AppendToMessageContent(AppendToMessageContent {
                            task_id: task_id.to_string(),
                            message: Some(message(
                                MessageKind::AgentOutput(AgentOutput { text }),
                                id.clone(),
                                task_id,
                                request_id,
                            )),
                            mask: Some(FieldMask {
                                paths: vec!["message.agent_output.text".to_string()],
                            }),
                        })
                    } else {
                        let id = Uuid::new_v4().to_string();
                        open_message_id = Some(id.clone());
                        Action::AddMessagesToTask(AddMessagesToTask {
                            task_id: task_id.to_string(),
                            messages: vec![message(
                                MessageKind::AgentOutput(AgentOutput { text }),
                                id,
                                task_id,
                                request_id,
                            )],
                        })
                    };
                    send_action(tx, action).await?;
                }
                if state.completed {
                    break 'stream;
                }
            }
        }
    }
    // A malformed call must prevent the entire batch from reaching the execution pipeline.
    let messages = state.tool_messages(task_id, request_id)?;
    if !messages.is_empty() {
        send_action(
            tx,
            Action::AddMessagesToTask(AddMessagesToTask {
                task_id: task_id.to_string(),
                messages,
            }),
        )
        .await?;
    }
    Ok(())
}
