use super::User;

#[test]
fn local_identity_has_a_stable_owner_without_account_metadata() {
    let first = User::local();
    let second = User::local();
    assert_eq!(first.local_id, second.local_id);
    assert_eq!(first.local_id.as_str(), "local-user");
    assert!(first.metadata.email.is_empty());
    assert!(first.metadata.photo_url.is_none());
    assert!(!first.needs_sso_link);
    assert!(!first.is_user_anonymous());
}

#[test]
fn test_user_global_skills_defaults_to_empty() {
    assert_eq!(User::test().global_skills, Vec::<String>::new());
}
