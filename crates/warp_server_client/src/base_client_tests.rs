use std::sync::Arc;

use futures::executor::block_on;
use warp_server_auth::auth_state::AuthState;

use super::{AuthenticatedGraphqlConfig, BaseClient, GraphqlRoutingConfig};

fn client() -> BaseClient {
    BaseClient::new(
        Arc::new(http_client::Client::new_for_test()),
        Arc::new(http_client::Client::new_for_test()),
        Arc::new(AuthState::new_for_test()),
        None,
        GraphqlRoutingConfig::default(),
        AuthenticatedGraphqlConfig::default(),
    )
}

#[test]
fn warp_account_requests_fail_without_starting_authentication() {
    let client = client();
    assert!(block_on(client.get_or_refresh_access_token()).is_err());
    assert!(block_on(client.graphql_request_options(None)).is_err());
    assert!(block_on(client.get_or_create_ambient_workload_token()).is_err());
    assert!(block_on(client.get_ambient_workload_token_valid_until(chrono::Utc::now())).is_err());
}
