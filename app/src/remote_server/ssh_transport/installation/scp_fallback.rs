use std::path::{Path, PathBuf};

use remote_server::setup::RemotePlatform;
use remote_server::transport::Error;

const REMOTE_SERVER_TARBALL_CACHE_FILE_NAME: &str = "oz.tar.gz";

/// Installs a cached archive by staging it on the remote host with SCP.
pub(super) async fn install(socket_path: &Path) -> Result<(), Error> {
    let platform = super::super::detect_remote_platform(socket_path).await?;

    let client_tarball_path = cached_remote_server_tarball(&platform)
        .await
        .map_err(Error::Other)?;
    let timeout = remote_server::setup::SCP_INSTALL_TIMEOUT;
    let install_dir = remote_server::setup::remote_server_dir();
    let remote_tarball_name = format!("oz-upload-{}.tar.gz", uuid::Uuid::new_v4());
    let remote_tarball_path = format!("{install_dir}/{remote_tarball_name}");

    let mkdir_output = remote_server::ssh::run_ssh_command(
        socket_path,
        &format!("mkdir -p {install_dir}"),
        remote_server::setup::CHECK_TIMEOUT,
    )
    .await
    .map_err(Error::from)?;
    if !mkdir_output.status.success() {
        let code = mkdir_output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&mkdir_output.stderr).to_string();
        return Err(Error::ScriptFailed {
            exit_code: code,
            stderr,
        });
    }

    log::info!("Uploading tarball to remote at {remote_tarball_path}");
    remote_server::ssh::scp_upload(
        socket_path,
        &client_tarball_path,
        &remote_tarball_path,
        timeout,
    )
    .await
    .map_err(Error::Other)?;

    log::info!("Running extraction via install script with tarball at {remote_tarball_path}");
    let script = remote_server::setup::install_script(&remote_tarball_path);

    let output = remote_server::ssh::run_ssh_script(socket_path, &script, timeout)
        .await
        .map_err(Error::from)?;
    if output.status.success() {
        Ok(())
    } else {
        let code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(Error::ScriptFailed {
            exit_code: code,
            stderr,
        })
    }
}

fn remote_server_tarball_cache_root() -> PathBuf {
    warp_core::paths::cache_dir()
        .join("remote-server")
        .join("tarballs")
}

fn current_remote_server_tarball_cache_version() -> &'static str {
    remote_server::setup::remote_server_artifact_version()
}

fn remote_server_tarball_cache_path(platform: &RemotePlatform) -> PathBuf {
    remote_server_tarball_cache_root()
        .join(current_remote_server_tarball_cache_version())
        .join(format!(
            "{}-{}",
            platform.os.as_str(),
            platform.arch.as_str()
        ))
        .join(REMOTE_SERVER_TARBALL_CACHE_FILE_NAME)
}

async fn is_valid_cached_tarball(path: &Path) -> bool {
    async_fs::metadata(path)
        .await
        .is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0)
}

/// Returns an existing archive for the remote platform.
async fn cached_remote_server_tarball(platform: &RemotePlatform) -> anyhow::Result<PathBuf> {
    let cache_path = remote_server_tarball_cache_path(platform);
    if is_valid_cached_tarball(&cache_path).await {
        log::info!(
            "Using cached remote-server tarball at {}",
            cache_path.display()
        );
        return Ok(cache_path);
    }

    anyhow::bail!(
        "No local remote-server archive found. Place a compatible {}-{} archive at '{}' \
         or manually deploy the remote binary to '{}' on the SSH host.",
        platform.os.as_str(),
        platform.arch.as_str(),
        cache_path.display(),
        remote_server::setup::remote_server_binary(),
    )
}
