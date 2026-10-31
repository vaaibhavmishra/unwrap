use std::time::Duration;

use futures::executor::block_on;
use vec1::vec1;
use warp_managed_secrets::client::{IdentityTokenOptions, ManagedSecretsClient as _};

use super::super::ServerApi;
use crate::server::ids::ServerId;
use crate::server::team_scope::RequestTeamScope;
use crate::workspaces::user_workspaces::TeamContextForOperation;

#[test]
fn managed_secrets_and_identity_tokens_fail_before_sending() {
    let mut server = warp_core::channel::ChannelState::mock_server();
    let request = server
        .mock("POST", "/graphql/v2")
        .match_query(mockito::Matcher::Any)
        .expect(0)
        .create();
    drop(server);
    let api = ServerApi::new_for_test();
    let scope =
        RequestTeamScope::from_scope(&TeamContextForOperation::new_for_test(ServerId::from(17)));
    for request_scope in [None, Some(&scope)] {
        assert!(block_on(api.list_secrets(request_scope)).is_err());
        assert!(
            block_on(api.issue_task_identity_token(
                request_scope,
                IdentityTokenOptions {
                    audience: "sts.amazonaws.com".into(),
                    requested_duration: Duration::from_secs(3600),
                    subject_template: vec1!["scoped_principal".into()],
                },
            ))
            .is_err()
        );
    }
    assert!(block_on(api.get_task_secrets("task-one".into(), "legacy-token".into())).is_err());
    request.assert();
}
