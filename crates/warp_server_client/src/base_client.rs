use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use instant::Duration;
use parking_lot::RwLock;
use warp_graphql::client::RequestOptions;
use warp_server_auth::auth_state::AuthState;

use crate::auth::UserUid;

/// Header key for the ambient agent workload token attached to authenticated requests.
pub const AMBIENT_WORKLOAD_TOKEN_HEADER: &str = "X-Warp-Ambient-Workload-Token";

/// Header key for the cloud agent task ID attached to ambient-agent requests.
pub const CLOUD_AGENT_ID_HEADER: &str = "X-Warp-Cloud-Agent-ID";

pub const PREWARMED_SANDBOX_ID_HEADER: &str = "X-Warp-Prewarmed-Sandbox-ID";
pub const PREWARMED_SANDBOX_WORKLOAD_TOKEN_HEADER: &str = "X-Warp-Prewarmed-Sandbox-Workload-Token";

/// Header used to communicate the source of an agent run.
pub const AGENT_SOURCE_HEADER: &str = "X-Oz-Api-Source";

/// Header carrying the request-local team scope for an operation whose team is inferred
/// from the current window rather than named explicitly in the request body. See
/// `specs/multi-team-api-context/TECH.md`. The server authenticates membership and rejects
/// a header that disagrees with the request body or an existing resource; adding this header
/// is not itself proof of membership.
pub const TEAM_UID_HEADER: &str = "X-Warp-Team-Uid";

/// Selects whether a contextual header is inherited, set, or omitted for one request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeaderOverride<T> {
    Inherit,
    Set(T),
    Omit,
}

/// Describes the request-local ambient agent headers that are safe to vary by endpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AmbientHeaderPolicy {
    pub workload_token: HeaderOverride<String>,
    pub cloud_agent_id: HeaderOverride<String>,
    pub agent_source: HeaderOverride<String>,
}

impl AmbientHeaderPolicy {
    /// Inherits every ambient agent contextual header configured on the client.
    pub fn inherit_all() -> Self {
        Self {
            workload_token: HeaderOverride::Inherit,
            cloud_agent_id: HeaderOverride::Inherit,
            agent_source: HeaderOverride::Inherit,
        }
    }

    /// Replaces only the cloud-agent task identifier for one task-scoped request.
    pub fn for_task(task_id: impl Into<String>) -> Self {
        Self {
            cloud_agent_id: HeaderOverride::Set(task_id.into()),
            ..Self::inherit_all()
        }
    }

    /// Includes workload-token context without cloud-agent or source context.
    pub fn workload_only() -> Self {
        Self {
            workload_token: HeaderOverride::Inherit,
            cloud_agent_id: HeaderOverride::Omit,
            agent_source: HeaderOverride::Omit,
        }
    }

    /// Omits all ambient agent contextual headers for a request.
    pub fn omit_all() -> Self {
        Self {
            workload_token: HeaderOverride::Omit,
            cloud_agent_id: HeaderOverride::Omit,
            agent_source: HeaderOverride::Omit,
        }
    }
}

impl Default for AmbientHeaderPolicy {
    fn default() -> Self {
        Self::inherit_all()
    }
}

/// Provides GraphQL path routing that applies independently of authentication.
#[derive(Clone, Debug, Default)]
pub struct GraphqlRoutingConfig {
    pub path_prefix: Option<String>,
}

/// Provides headers added only to session-authenticated GraphQL operations.
#[derive(Clone, Debug, Default)]
pub struct AuthenticatedGraphqlConfig {
    pub headers: HashMap<String, String>,
}

#[derive(Clone)]
enum AmbientIdentity {
    Run(String),
    Standby(String),
}

/// Owns shared transport, authentication, and authenticated request decoration.
pub struct BaseClient {
    client: Arc<http_client::Client>,
    external_client: Arc<http_client::Client>,
    auth_state: Arc<AuthState>,
    ambient_identity: RwLock<Option<AmbientIdentity>>,
    agent_source: Option<String>,
    graphql_routing: GraphqlRoutingConfig,
    authenticated_graphql: AuthenticatedGraphqlConfig,
}

impl BaseClient {
    pub fn new(
        client: Arc<http_client::Client>,
        external_client: Arc<http_client::Client>,
        auth_state: Arc<AuthState>,
        agent_source: Option<String>,
        graphql_routing: GraphqlRoutingConfig,
        mut authenticated_graphql: AuthenticatedGraphqlConfig,
    ) -> Self {
        authenticated_graphql.headers.retain(|name, _| {
            if Self::is_reserved_authenticated_graphql_header(name) {
                log::warn!("Ignoring reserved authenticated GraphQL header configuration: {name}");
                false
            } else {
                true
            }
        });
        Self {
            client,
            external_client,
            auth_state,
            ambient_identity: RwLock::new(None),
            agent_source,
            graphql_routing,
            authenticated_graphql,
        }
    }

    /// Returns whether authenticated GraphQL decoration would override BaseClient-owned headers.
    fn is_reserved_authenticated_graphql_header(name: &str) -> bool {
        [
            http::header::AUTHORIZATION.as_str(),
            http::header::CONTENT_TYPE.as_str(),
            http::header::CONTENT_LENGTH.as_str(),
            http_client::iap::IAP_PROXY_AUTH_HEADER,
            AMBIENT_WORKLOAD_TOKEN_HEADER,
            CLOUD_AGENT_ID_HEADER,
            PREWARMED_SANDBOX_ID_HEADER,
            PREWARMED_SANDBOX_WORKLOAD_TOKEN_HEADER,
            AGENT_SOURCE_HEADER,
            TEAM_UID_HEADER,
        ]
        .iter()
        .any(|reserved| name.eq_ignore_ascii_case(reserved))
    }

    /// Returns the shared HTTP client for request construction.
    pub fn http_client(&self) -> &http_client::Client {
        self.client.as_ref()
    }

    /// Returns the pooled external HTTP client.
    pub fn external_http_client(&self) -> &http_client::Client {
        self.external_client.as_ref()
    }

    /// Returns an owned handle to the shared HTTP client for GraphQL operations.
    pub fn owned_http_client(&self) -> Arc<http_client::Client> {
        self.client.clone()
    }

    pub fn anonymous_id(&self) -> String {
        self.auth_state.anonymous_id()
    }

    pub fn user_id(&self) -> Option<UserUid> {
        self.auth_state.user_id()
    }

    /// Returns whether the authenticated principal is a service account.
    pub fn is_service_account(&self) -> bool {
        self.auth_state.is_service_account()
    }

    pub async fn get_or_refresh_access_token(&self) -> Result<Option<String>> {
        anyhow::bail!("Warp account authentication is not available in Unwrap")
    }

    /// Sets the default cloud-agent identifier inherited by subsequent requests.
    pub fn set_ambient_agent_task_id(&self, task_id: Option<String>) {
        *self.ambient_identity.write() = task_id.map(AmbientIdentity::Run);
    }

    /// Selects standby identity for subsequent authenticated GraphQL requests.
    pub fn set_prewarmed_sandbox_id(&self, sandbox_id: String) {
        *self.ambient_identity.write() = Some(AmbientIdentity::Standby(sandbox_id));
    }
    pub async fn get_or_create_ambient_workload_token(&self) -> Result<Option<String>> {
        anyhow::bail!("Warp cloud workload authentication is not available in Unwrap")
    }

    pub async fn get_ambient_workload_token_valid_until(
        &self,
        must_outlive: DateTime<Utc>,
    ) -> Result<Option<String>> {
        anyhow::bail!(
            "Warp cloud workload authentication through {must_outlive} is not available in Unwrap"
        )
    }

    /// Resolves request-local ambient agent policy into wire headers.
    pub async fn ambient_headers(
        &self,
        policy: AmbientHeaderPolicy,
    ) -> Result<Vec<(String, String)>> {
        let workload_token = match policy.workload_token {
            HeaderOverride::Inherit => self
                .get_or_create_ambient_workload_token()
                .await
                .context("Failed to get ambient agent workload token")?,
            HeaderOverride::Set(token) => Some(token),
            HeaderOverride::Omit => None,
        };
        let cloud_agent_id = match policy.cloud_agent_id {
            HeaderOverride::Inherit => match self.ambient_identity.read().as_ref() {
                Some(AmbientIdentity::Run(task_id)) => Some(task_id.clone()),
                Some(AmbientIdentity::Standby(_)) | None => None,
            },
            HeaderOverride::Set(task_id) => Some(task_id),
            HeaderOverride::Omit => None,
        };
        let agent_source = match policy.agent_source {
            HeaderOverride::Inherit => self.agent_source.clone(),
            HeaderOverride::Set(source) => Some(source),
            HeaderOverride::Omit => None,
        };

        Ok(workload_token
            .map(|token| (AMBIENT_WORKLOAD_TOKEN_HEADER.to_string(), token))
            .into_iter()
            .chain(cloud_agent_id.map(|id| (CLOUD_AGENT_ID_HEADER.to_string(), id)))
            .chain(agent_source.map(|source| (AGENT_SOURCE_HEADER.to_string(), source)))
            .collect())
    }

    /// Returns GraphQL options for bootstrap or explicit-token operations.
    pub fn graphql_request_options_with_token(&self, auth_token: Option<String>) -> RequestOptions {
        RequestOptions {
            auth_token,
            path_prefix: self.graphql_routing.path_prefix.clone(),
            ..RequestOptions::default()
        }
    }

    /// Returns GraphQL options for a session-authenticated operation.
    pub async fn graphql_request_options(
        &self,
        timeout: Option<Duration>,
    ) -> Result<RequestOptions> {
        let auth_token = self
            .get_or_refresh_access_token()
            .await
            .context("Failed to get access token for GraphQL request")?;
        let mut options = self.graphql_request_options_with_token(auth_token);
        options.timeout = timeout;
        options.headers = self.authenticated_graphql.headers.clone();
        let identity = self.ambient_identity.read().clone();
        match identity {
            Some(AmbientIdentity::Standby(sandbox_id)) => {
                let token = self
                    .get_or_create_ambient_workload_token()
                    .await?
                    .context("A standby GraphQL request requires a workload token")?;
                options
                    .headers
                    .insert(PREWARMED_SANDBOX_ID_HEADER.to_string(), sandbox_id);
                options
                    .headers
                    .insert(PREWARMED_SANDBOX_WORKLOAD_TOKEN_HEADER.to_string(), token);
            }
            Some(AmbientIdentity::Run(task_id)) => {
                options.headers.extend(
                    self.ambient_headers(AmbientHeaderPolicy::for_task(task_id))
                        .await?,
                );
            }
            None => {
                options.headers.extend(
                    self.ambient_headers(AmbientHeaderPolicy::inherit_all())
                        .await?,
                );
            }
        }
        Ok(options)
    }
}

#[cfg(test)]
#[path = "base_client_tests.rs"]
mod tests;
