use std::borrow::Cow;

use anyhow::{Result, anyhow};
use instant::Duration;
use warp_graphql::client::{Operation, RequestOptions};
use warpui_core::r#async::BoxFuture;

use crate::base_client::{BaseClient, TEAM_UID_HEADER};

/// Sends a GraphQL operation through a base client supplied by the application.
///
/// This function is deliberately generic so concrete endpoint operation
/// instantiations occur in server client crates rather than in the app crate.
pub fn send_graphql_request<'a, QF: 'a, O>(
    base_client: &'a BaseClient,
    operation: O,
    timeout: Option<Duration>,
) -> BoxFuture<'a, Result<QF>>
where
    O: Operation<QF> + Send + 'a,
{
    Box::pin(async move {
        let options = base_client.graphql_request_options(timeout).await?;
        send_graphql_request_with_options(base_client, operation, options).await
    })
}

pub fn send_team_scoped_graphql_request<'a, QF: 'a, O>(
    base_client: &'a BaseClient,
    operation: O,
    timeout: Option<Duration>,
    team_uid: String,
) -> BoxFuture<'a, Result<QF>>
where
    O: Operation<QF> + Send + 'a,
{
    Box::pin(async move {
        let mut options = base_client.graphql_request_options(timeout).await?;
        options
            .headers
            .insert(TEAM_UID_HEADER.to_string(), team_uid);
        send_graphql_request_with_options(base_client, operation, options).await
    })
}

pub(crate) async fn send_graphql_request_with_options<QF, O>(
    base_client: &BaseClient,
    operation: O,
    options: RequestOptions,
) -> Result<QF>
where
    O: Operation<QF> + Send,
{
    let operation_name = operation.operation_name().map(Cow::into_owned);
    let response = operation
        .send_request(base_client.owned_http_client(), options)
        .await?;

    response.data.ok_or_else(|| {
        let operation_label = operation_name
            .as_deref()
            .unwrap_or("unknown GraphQL operation");
        let error_messages = response
            .errors
            .as_ref()
            .map(|errors| {
                errors
                    .iter()
                    .filter_map(|error| {
                        let message = error.message.trim();
                        (!message.is_empty()).then(|| message.to_string())
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .filter(|messages| !messages.is_empty());
        match error_messages {
            Some(messages) => {
                anyhow!("missing response data for {operation_label}: {messages}")
            }
            None => anyhow!("missing response data for {operation_label}"),
        }
    })
}

#[cfg(test)]
#[path = "graphql_helpers_tests.rs"]
mod tests;
