use std::sync::Arc;

use futures::executor::block_on;
use warp_errors::AnyhowErrorExt as _;
use warp_server_auth::auth_state::AuthState;

use super::HttpStatusError;
use crate::base_client::{
    AMBIENT_WORKLOAD_TOKEN_HEADER, AuthenticatedGraphqlConfig, BaseClient, CLOUD_AGENT_ID_HEADER,
    GraphqlRoutingConfig,
};

#[test]
fn presigned_download_keeps_api_context_off_storage() {
    let mut storage = mockito::Server::new();
    let query = "X-Amz-Signature=test%2Bsignature&key=a%2Fb&key=c+%20d";
    let url = format!("{}/snapshot?{query}", storage.url());
    let mut storage_request = storage
        .mock("GET", "/snapshot")
        .match_query(mockito::Matcher::Exact(query.into()))
        .with_status(200)
        .with_body("snapshot bytes");
    for header in [
        "x-oz-api-source",
        "x-warp-client-id",
        "x-warp-client-version",
        "x-warp-os-category",
        "x-warp-os-name",
        "x-warp-os-version",
        "x-warp-os-linux-kernel-version",
        "x-warp-traceparent",
        "authorization",
        "proxy-authorization",
        "cookie",
        CLOUD_AGENT_ID_HEADER,
        AMBIENT_WORKLOAD_TOKEN_HEADER,
    ] {
        storage_request = storage_request.match_header(header, mockito::Matcher::Missing);
    }
    let storage_request = storage_request.create();
    let auth_state = AuthState::new_logged_out_for_test();
    auth_state.set_remote_server_bearer_token("bearer-token".into());
    let mut defaults = http::HeaderMap::new();
    for (name, value) in [
        ("authorization", "Bearer test-default-auth"),
        ("proxy-authorization", "Bearer test-default-iap"),
        ("cookie", "session=test-default-cookie"),
        ("x-warp-client-id", "test-default-client"),
        ("x-warp-client-version", "test-default-version"),
        ("x-warp-os-category", "test-default-os"),
        ("x-warp-os-name", "test-default-os-name"),
        ("x-oz-api-source", "test-default-source"),
        (AMBIENT_WORKLOAD_TOKEN_HEADER, "test-default-workload"),
        (CLOUD_AGENT_ID_HEADER, "test-default-task"),
        ("x-test-client-default", "api-only"),
    ] {
        defaults.insert(
            http::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
            http::HeaderValue::from_static(value),
        );
    }
    let api_http_client = http_client::Client::from_client_builder(
        reqwest::Client::builder().default_headers(defaults),
    )
    .unwrap();
    let client = BaseClient::new(
        Arc::new(api_http_client),
        Arc::new(http_client::Client::new_external()),
        Arc::new(auth_state),
        Some("cloud-mode".into()),
        GraphqlRoutingConfig::default(),
        AuthenticatedGraphqlConfig::default(),
    );
    let bytes = block_on(client.download_presigned(&url)).unwrap();
    assert_eq!(bytes.as_ref(), b"snapshot bytes");
    storage_request.assert();
}

#[test]
fn shared_status_error_actionability_ignores_retryable_client_failures() {
    let error = anyhow::Error::new(HttpStatusError::new(429, "retry later".to_string()))
        .context("Public API request failed");

    assert!(!error.is_actionable());
}
