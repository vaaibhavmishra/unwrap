use remote_server::auth::RemoteServerAuthContext;
use warpui::r#async::BoxFuture;

/// Local identity used to reconnect to the same SSH-side daemon.
pub fn local_auth_context(crash_reporting_enabled: bool) -> RemoteServerAuthContext {
    RemoteServerAuthContext::new(
        || -> BoxFuture<'static, Option<String>> { Box::pin(async { None }) },
        || "local-user".to_owned(),
        "local-user".to_owned(),
        String::new(),
        crash_reporting_enabled,
    )
}
