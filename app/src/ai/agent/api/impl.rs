use super::{ConvertToAPITypeError, RequestParams, ResponseStream};
use crate::ai::agent::redaction;

pub async fn generate_multi_agent_output(
    mut params: RequestParams,
    cancellation_rx: futures::channel::oneshot::Receiver<()>,
) -> Result<ResponseStream, ConvertToAPITypeError> {
    if params.should_redact_secrets {
        redaction::redact_inputs(&mut params.input);
    }
    super::local_byok::generate_local_byok_output(params, cancellation_rx).await
}
