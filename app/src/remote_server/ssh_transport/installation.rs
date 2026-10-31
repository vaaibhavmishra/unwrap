#[path = "installation/scp_fallback.rs"]
mod scp_fallback;

use std::path::Path;

use remote_server::transport::{Error, InstallOutcome, InstallSource};

/// Installs a locally cached remote-server archive over SSH.
pub(super) async fn install_binary(socket_path: &Path) -> InstallOutcome {
    let binary_path = remote_server::setup::remote_server_binary();
    log::info!("Installing remote server binary to {binary_path}");
    let mut outcome = InstallOutcome {
        source: Some(InstallSource::Client),
        result: scp_fallback::install(socket_path).await,
    };

    // Post-install verification: confirm the binary actually landed at the
    // expected path and is functional. This catches silent install failures
    // that would otherwise surface as a cryptic IPC handshake error.
    if outcome.result.is_ok() {
        log::info!("Running post-install verification for {binary_path}");
        let check_cmd = remote_server::setup::binary_check_command();
        let verify = remote_server::ssh::run_ssh_command(
            socket_path,
            &check_cmd,
            remote_server::setup::CHECK_TIMEOUT,
        )
        .await;
        match verify {
            Ok(output) if output.status.success() => {}
            Ok(output) => {
                let code = output.status.code().unwrap_or(-1);
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                outcome.result = Err(Error::Other(anyhow::anyhow!(
                    "Post-install verification failed: binary not found or not \
                     executable at {binary_path} (exit {code}): {stderr}"
                )));
            }
            Err(e) => {
                outcome.result = Err(Error::Other(anyhow::anyhow!(
                    "Post-install verification failed: {e}"
                )));
            }
        }
    }

    outcome
}
