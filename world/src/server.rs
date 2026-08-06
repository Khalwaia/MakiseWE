use std::future::Future;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use makise_proto::v1::world_service_server::WorldServiceServer;
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::Server;

use crate::{PathGuard, WorldRpc};

pub async fn serve_uds(
    socket_path: impl AsRef<Path>,
    rpc: WorldRpc,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> anyhow::Result<()> {
    let guard = PathGuard::default();
    let socket_path = guard.validate(socket_path)?;
    let parent = socket_path
        .parent()
        .context("Unix socket path has no parent")?;
    guard.validate(parent)?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("failed to create {}", parent.display()))?;
    if socket_path.exists() {
        bail!(
            "refusing to replace existing Unix socket path {}",
            socket_path.display()
        );
    }

    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("failed to bind {}", socket_path.display()))?;
    std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600))
        .with_context(|| format!("failed to secure {}", socket_path.display()))?;
    let socket_guard = BoundSocket::new(socket_path)?;

    let result = Server::builder()
        .add_service(WorldServiceServer::new(rpc))
        .serve_with_incoming_shutdown(UnixListenerStream::new(listener), shutdown)
        .await
        .context("WorldService stopped with an error");
    drop(socket_guard);
    result
}

struct BoundSocket {
    path: PathBuf,
    device: u64,
    inode: u64,
}

impl BoundSocket {
    fn new(path: PathBuf) -> anyhow::Result<Self> {
        let metadata = std::fs::symlink_metadata(&path)
            .with_context(|| format!("failed to inspect {}", path.display()))?;
        if !metadata.file_type().is_socket() {
            bail!("bound UDS path is not a socket: {}", path.display());
        }
        Ok(Self {
            path,
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }
}

impl Drop for BoundSocket {
    fn drop(&mut self) {
        let Ok(metadata) = std::fs::symlink_metadata(&self.path) else {
            return;
        };
        if metadata.file_type().is_socket()
            && metadata.dev() == self.device
            && metadata.ino() == self.inode
        {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
