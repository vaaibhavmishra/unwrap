use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use uuid::Uuid;
use warpui_core::{AppContext, Entity, SingletonEntity};

use super::UserUid;
use super::anonymous_id::get_or_create_anonymous_id;
#[cfg(any(not(target_family = "wasm"), test, feature = "test-util"))]
use super::user::UserMetadata;
use super::user::{AnonymousUserType, PersonalObjectLimits, PrincipalType, User};

const ANONYMOUS_USER_NOTIFICATION_BLOCK_TIMER: Duration = Duration::days(7);

/// Local identity and compatibility metadata for persisted objects.
pub struct AuthState {
    /// Identity metadata used for local object ownership.
    user: RwLock<Option<User>>,

    /// An anonymous UUID. Can be used to consistently identify an anonymous user who is not logged in.
    anonymous_id: Uuid,

    /// Credential for the local remote-server peer protocol.
    remote_peer_token: RwLock<Option<String>>,
    #[cfg(any(
        test,
        feature = "integration_tests",
        feature = "skip_login",
        feature = "test-util"
    ))]
    test_authenticated: bool,
}

impl AuthState {
    fn new(ctx: &AppContext) -> Self {
        Self {
            user: RwLock::new(None),
            anonymous_id: get_or_create_anonymous_id(ctx),
            remote_peer_token: RwLock::new(None),
            #[cfg(any(
                test,
                feature = "integration_tests",
                feature = "skip_login",
                feature = "test-util"
            ))]
            test_authenticated: false,
        }
    }
    #[cfg(any(test, feature = "integration_tests", feature = "test-util"))]
    pub fn new_for_test() -> Self {
        Self {
            user: RwLock::new(Some(User::test())),
            anonymous_id: Uuid::new_v4(),
            remote_peer_token: RwLock::new(None),
            test_authenticated: true,
        }
    }
    #[cfg(any(test, feature = "test-util"))]
    pub fn new_logged_out_for_test() -> Self {
        Self {
            user: RwLock::new(None),
            anonymous_id: Uuid::new_v4(),
            remote_peer_token: RwLock::new(None),
            #[cfg(any(
                test,
                feature = "integration_tests",
                feature = "skip_login",
                feature = "test-util"
            ))]
            test_authenticated: false,
        }
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn new_anonymous_for_test() -> Self {
        use super::user::AnonymousUserType;
        Self {
            user: RwLock::new(Some(User {
                anonymous_user_type: Some(AnonymousUserType::NativeClientAnonymousUserFeatureGated),
                ..User::test()
            })),
            anonymous_id: Uuid::new_v4(),
            remote_peer_token: RwLock::new(None),
            test_authenticated: true,
        }
    }

    /// Creates the local identity without reading or restoring Warp credentials.
    pub fn initialize(ctx: &AppContext) -> Self {
        let state = Self::new(ctx);
        state.set_user(Some(User::local()));
        state
    }

    /// Replaces the identity metadata associated with local objects.
    pub fn set_user(&self, user: Option<User>) {
        *self.user.write() = user;
    }

    /// Applies auth data received by the remote server daemon handshake.
    ///
    /// Empty values are authoritative: an empty token clears bearer credentials, and an empty user
    /// ID clears the daemon user identity.
    #[cfg(any(not(target_family = "wasm"), test, feature = "test-util"))]
    pub fn apply_remote_server_auth_context(
        &self,
        auth_token: String,
        user_id: String,
        user_email: String,
    ) {
        self.set_remote_server_bearer_token(auth_token);
        self.set_remote_server_user(user_id, user_email);
    }

    /// Applies bearer-token credentials received from the remote server daemon.
    #[cfg(any(not(target_family = "wasm"), test, feature = "test-util"))]
    pub fn set_remote_server_bearer_token(&self, auth_token: String) {
        *self.remote_peer_token.write() = (!auth_token.is_empty()).then_some(auth_token);
    }

    #[cfg(any(not(target_family = "wasm"), test, feature = "test-util"))]
    fn set_remote_server_user(&self, user_id: String, user_email: String) {
        let mut user = self.user.write();
        if user_id.is_empty() {
            *user = None;
            return;
        }

        match user.as_mut() {
            Some(user) => {
                user.local_id = UserUid::new(&user_id);
                user.metadata.email = user_email;
            }
            None => {
                *user = Some(User {
                    local_id: UserUid::new(&user_id),
                    metadata: UserMetadata {
                        email: user_email,
                        display_name: None,
                        photo_url: None,
                    },
                    is_onboarded: false,
                    needs_sso_link: false,
                    anonymous_user_type: None,
                    is_on_work_domain: false,
                    linked_at: None,
                    personal_object_limits: None,
                    principal_type: PrincipalType::default(),
                    global_skills: Vec::new(),
                });
            }
        }
    }

    /// Returns whether an authenticated test fixture is installed.
    pub fn is_logged_in(&self) -> bool {
        #[cfg(any(
            test,
            feature = "integration_tests",
            feature = "skip_login",
            feature = "test-util"
        ))]
        {
            self.test_authenticated
                && self.user.read().is_some()
                && self.remote_peer_token.read().is_none()
        }
        #[cfg(not(any(
            test,
            feature = "integration_tests",
            feature = "skip_login",
            feature = "test-util"
        )))]
        {
            false
        }
    }

    /// Returns whether the user should be treated as not having a full account.
    /// True if the user is anonymous OR if there is no user at all (fully logged out).
    ///
    /// Note: uses `unwrap_or(true)` intentionally (not `unwrap_or_default()`) so that
    /// during the transient state where credentials exist but user data hasn't loaded
    /// yet, the user is conservatively treated as lacking a full account.
    pub fn is_anonymous_or_logged_out(&self) -> bool {
        !self.is_logged_in() || self.is_user_anonymous().unwrap_or(true)
    }

    /// Returns the peer credential used to validate local remote-server protocol requests.
    pub fn remote_server_auth_token(&self) -> Option<String> {
        self.remote_peer_token.read().clone()
    }

    /// Returns the user's display name.
    pub fn username_for_display(&self) -> Option<String> {
        Some(self.user.read().as_ref()?.username_for_display().to_owned())
    }

    /// Returns the user's display name, does NOT fall back to email.
    pub fn display_name(&self) -> Option<String> {
        self.user
            .read()
            .as_ref()
            .and_then(|user| user.display_name().to_owned())
    }

    /// Returns the user's email. Note the non-obvious semantics of this function:
    /// If the user is logged in and not anonymous, the email will always be populated.
    /// If the user is logged in and anonymous, their email will be an empty string.
    /// If the user is not logged in, their email will be `None`.
    pub fn user_email(&self) -> Option<String> {
        self.user
            .read()
            .as_ref()
            .map(|user| user.metadata.email.clone())
    }

    /// Returns whether the user considered onboarded to Warp.
    pub fn is_onboarded(&self) -> Option<bool> {
        self.user.read().as_ref().map(|user| user.is_onboarded)
    }

    /// Returns the user's email domain (anything after the @ sign of their email).
    pub fn user_email_domain(&self) -> Option<String> {
        self.user.read().as_ref().map(|user| {
            user.metadata
                .email
                .clone()
                .split('@')
                .nth(1)
                .unwrap_or("")
                .to_string()
        })
    }

    /// Returns whether or not the user is anonymous.
    /// Returns `None` if there is no user data.
    pub fn is_user_anonymous(&self) -> Option<bool> {
        self.user
            .read()
            .as_ref()
            .map(|user| user.is_user_anonymous())
    }

    /// Returns whether or not the user is a "web client anonymous user", aka their account
    /// originated from viewing Warp on web.
    pub fn is_user_web_anonymous_user(&self) -> Option<bool> {
        self.user.read().as_ref().map(|user| {
            user.anonymous_user_type() == Some(AnonymousUserType::WebClientAnonymousUser)
                && user.linked_at().is_none()
        })
    }

    /// Returns whether or not the user is a feature gated anonymous user.
    pub fn is_anonymous_user_feature_gated(&self) -> Option<bool> {
        self.user.read().as_ref().map(|user| {
            if !self.is_user_anonymous().unwrap_or_default() {
                return false;
            }

            matches!(
                user.anonymous_user_type(),
                Some(AnonymousUserType::NativeClientAnonymousUserFeatureGated)
            )
        })
    }

    /// Returns the cached profile picture URL.
    pub fn user_photo_url(&self) -> Option<String> {
        self.user
            .read()
            .as_ref()
            .and_then(|user| user.metadata.photo_url.clone())
    }

    /// Returns whether or not the user needs to link their account to an SSO provider.
    pub fn needs_sso_link(&self) -> Option<bool> {
        self.user.read().as_ref().map(|user| user.needs_sso_link)
    }

    /// Returns the anonymous user type.
    /// Note that a `Some()` value here does NOT mean the user is still anonymous;
    /// they might have since signed up, but we keep their anonymous user type around.
    pub fn anonymous_user_type(&self) -> Option<AnonymousUserType> {
        self.user
            .read()
            .as_ref()
            .and_then(|user| user.anonymous_user_type())
    }

    /// Returns the personal object limits the user has.
    /// Currently, only anonymous users have limits.
    pub fn personal_object_limits(&self) -> Option<PersonalObjectLimits> {
        self.user
            .read()
            .as_ref()
            .and_then(|user| user.personal_object_limits())
    }

    /// Set whether or not the user is onboarded.
    pub fn set_is_onboarded(&self, is_onboarded: bool) {
        if let Some(user) = self.user.write().as_mut() {
            user.is_onboarded = is_onboarded;
        }
    }

    /// Returns the owner identifier associated with the local identity.
    pub fn user_id(&self) -> Option<UserUid> {
        self.user.read().as_ref().map(|user| user.local_id)
    }

    /// Returns the user's anonymous id.
    /// The anonymous id will be consistent across the app's lifetime. It is a random UUID.
    pub fn anonymous_id(&self) -> String {
        self.anonymous_id.to_string()
    }

    /// Returns whether or not the renotification block to encourage anonymous users to sign up
    /// has expired.
    pub fn anonymous_user_renotification_block_expired(
        &self,
        last_time_opt: Option<String>,
    ) -> bool {
        self.is_anonymous_user_feature_gated().unwrap_or_default()
            && last_time_opt
                .and_then(|last_time_string| last_time_string.parse::<DateTime<Utc>>().ok())
                .is_none_or(|last_time| {
                    Utc::now() - ANONYMOUS_USER_NOTIFICATION_BLOCK_TIMER >= last_time
                })
    }

    /// Returns whether or not the user is on a work domain.
    /// This calculation is done on the server, using a list of
    pub fn is_on_work_domain(&self) -> Option<bool> {
        self.user.read().as_ref().map(|user| user.is_on_work_domain)
    }

    /// Returns the type of principal (user or service account).
    pub fn principal_type(&self) -> Option<PrincipalType> {
        self.user.read().as_ref().map(|user| user.principal_type)
    }

    /// Returns whether the authenticated principal is a service account.
    pub fn is_service_account(&self) -> bool {
        matches!(self.principal_type(), Some(PrincipalType::ServiceAccount))
    }

    /// Returns the cached global skill specs for the current user.
    pub fn global_skills(&self) -> Vec<String> {
        self.user
            .read()
            .as_ref()
            .map(|user| user.global_skills.clone())
            .unwrap_or_default()
    }
}

// Adapter for the [`warp_managed_secrets`] crate, which needs to access the current user.
impl warp_managed_secrets::ActorProvider for AuthState {
    fn actor_uid(&self) -> Option<String> {
        self.user_id().map(|uid| uid.as_string())
    }
}

/// AuthStateProvider is a singleton model which provides a reference to the global AuthState.
pub struct AuthStateProvider {
    auth_state: Arc<AuthState>,
}

impl AuthStateProvider {
    pub fn new(auth_state: Arc<AuthState>) -> Self {
        Self { auth_state }
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn new_for_test() -> Self {
        Self {
            auth_state: Arc::new(AuthState::new_for_test()),
        }
    }

    /// Constructs a provider backed by a fully logged-out `AuthState` (no user,
    /// no credentials). Used by unit tests that need to exercise code paths
    /// gated on `AuthState::user_id()` / `UserWorkspaces::personal_drive()`
    /// returning `None`.
    #[cfg(any(test, feature = "test-util"))]
    pub fn new_logged_out_for_test() -> Self {
        Self {
            auth_state: Arc::new(AuthState::new_logged_out_for_test()),
        }
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn new_anonymous_for_test() -> Self {
        Self {
            auth_state: Arc::new(AuthState::new_anonymous_for_test()),
        }
    }

    pub fn get(&self) -> &Arc<AuthState> {
        &self.auth_state
    }
}

impl Entity for AuthStateProvider {
    type Event = ();
}

impl SingletonEntity for AuthStateProvider {}

#[cfg(test)]
#[path = "auth_state_tests.rs"]
mod tests;
