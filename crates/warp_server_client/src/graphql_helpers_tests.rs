use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures::executor::block_on;
use warp_graphql::client::{GraphQLError, RequestOptions};
use warp_server_auth::auth_state::AuthState;
use warpui_core::r#async::BoxFuture;

use super::{send_graphql_request, send_team_scoped_graphql_request};
use crate::base_client::{AuthenticatedGraphqlConfig, BaseClient, GraphqlRoutingConfig};

struct RecordingOperation(Arc<AtomicUsize>);

impl warp_graphql::client::Operation<()> for RecordingOperation {
    fn operation_name(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed("RecordingOperation"))
    }

    fn send_request(
        self,
        client: Arc<http_client::Client>,
        options: RequestOptions,
    ) -> BoxFuture<'static, Result<cynic::GraphQlResponse<()>, GraphQLError>> {
        drop((client, options));
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            Ok(cynic::GraphQlResponse {
                data: Some(()),
                errors: None,
            })
        })
    }
}

#[test]
fn backend_operations_cannot_send_requests_even_with_legacy_credentials() {
    let auth_state = AuthState::new_logged_out_for_test();
    auth_state.set_remote_server_bearer_token("legacy-token".to_string());
    let client = BaseClient::new(
        Arc::new(http_client::Client::new_for_test()),
        Arc::new(http_client::Client::new_for_test()),
        Arc::new(auth_state),
        None,
        GraphqlRoutingConfig::default(),
        AuthenticatedGraphqlConfig::default(),
    );
    let sent = Arc::new(AtomicUsize::new(0));
    assert!(
        block_on(send_graphql_request(
            &client,
            RecordingOperation(sent.clone()),
            None
        ))
        .is_err()
    );
    assert!(
        block_on(send_team_scoped_graphql_request(
            &client,
            RecordingOperation(sent.clone()),
            None,
            "legacy-team".to_string(),
        ))
        .is_err()
    );
    assert_eq!(sent.load(Ordering::SeqCst), 0);
}
