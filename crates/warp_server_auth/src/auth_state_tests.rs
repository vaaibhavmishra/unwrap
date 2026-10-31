use super::AuthState;
use crate::user::User;

#[test]
fn peer_credentials_do_not_enable_login_or_change_local_ownership() {
    let state = AuthState::new_logged_out_for_test();
    state.set_user(Some(User::local()));
    let owner = state.user_id();
    assert_eq!(owner.unwrap().as_str(), "local-user");
    state.set_remote_server_bearer_token("peer-token".into());
    assert!(!state.is_logged_in());
    assert!(state.is_anonymous_or_logged_out());
    assert_eq!(state.user_id(), owner);
}

#[test]
fn remote_peer_credentials_remain_local_and_do_not_sign_in() {
    let state = AuthState::new_logged_out_for_test();
    state.apply_remote_server_auth_context("peer-token".into(), "local-user".into(), String::new());
    assert_eq!(
        state.remote_server_auth_token().as_deref(),
        Some("peer-token")
    );
    assert_eq!(state.user_id().unwrap().as_str(), "local-user");
    assert!(!state.is_logged_in());
    state.set_remote_server_bearer_token(String::new());
    assert!(state.remote_server_auth_token().is_none());
    assert!(!state.is_logged_in());
}

#[test]
fn local_identity_has_no_peer_credentials() {
    let state = AuthState::new_logged_out_for_test();
    state.set_user(Some(User::local()));
    assert!(state.remote_server_auth_token().is_none());
    assert!(!state.is_logged_in());
}
