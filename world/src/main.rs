use std::path::PathBuf;

use anyhow::{Context, bail};
use makise_world::{
    PathGuard, WorldActorConfig, WorldActorHandle, WorldDefinition, WorldEngine, WorldRpc,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let Some(command) = arguments.next() else {
        print_usage();
        return Ok(());
    };

    match command.to_string_lossy().as_ref() {
        "verify-package" => {
            let manifest = required_path(arguments.next(), "manifest path")?;
            let definition = WorldDefinition::load(&manifest, &PathGuard::default())
                .with_context(|| format!("failed to load {}", manifest.display()))?;
            println!(
                "world package is valid: id={} hash={}",
                definition.world_id(),
                definition.hash()
            );
        }
        "status" => {
            let database = required_path(arguments.next(), "database path")?;
            let manifest = required_path(arguments.next(), "manifest path")?;
            let identity = required_string(arguments.next(), "identity ID")?;
            let initial_anchor = required_string(arguments.next(), "initial anchor")?;
            let now_ms = unix_time_ms()?;
            let guard = PathGuard::default();
            let definition = WorldDefinition::load(manifest, &guard)?;
            let engine = WorldEngine::open(
                database,
                &identity,
                definition,
                &initial_anchor,
                now_ms,
                &guard,
            )?;
            println!("{}", serde_json::to_string_pretty(&engine.perception()?)?);
        }
        "serve" => {
            serve(arguments).await?;
        }
        other => bail!("unknown command {other:?}"),
    }
    Ok(())
}

#[cfg(unix)]
async fn serve(mut arguments: impl Iterator<Item = std::ffi::OsString>) -> anyhow::Result<()> {
    let socket = required_path(arguments.next(), "Unix socket path")?;
    let database = required_path(arguments.next(), "database path")?;
    let manifest = required_path(arguments.next(), "manifest path")?;
    let identity = required_string(arguments.next(), "identity ID")?;
    let initial_anchor = required_string(arguments.next(), "initial anchor")?;

    let now_ms = unix_time_ms()?;
    let guard = PathGuard::default();
    let definition = WorldDefinition::load(manifest, &guard)?;
    let engine = WorldEngine::open(
        database,
        &identity,
        definition,
        &initial_anchor,
        now_ms,
        &guard,
    )?;
    let actor = WorldActorHandle::spawn(engine, WorldActorConfig::default())?;
    let rpc = WorldRpc::new(actor);
    makise_world::serve_uds(socket, rpc, async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await
}

#[cfg(not(unix))]
async fn serve(_arguments: impl Iterator<Item = std::ffi::OsString>) -> anyhow::Result<()> {
    bail!("serve requires a Unix platform with Unix Domain Socket support")
}

fn required_path(value: Option<std::ffi::OsString>, name: &str) -> anyhow::Result<PathBuf> {
    value
        .map(PathBuf::from)
        .with_context(|| format!("missing {name}"))
}

fn required_string(value: Option<std::ffi::OsString>, name: &str) -> anyhow::Result<String> {
    Ok(value
        .with_context(|| format!("missing {name}"))?
        .to_string_lossy()
        .into_owned())
}

fn unix_time_ms() -> anyhow::Result<i64> {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .context("system clock is before Unix epoch")?;
    i64::try_from(duration.as_millis()).context("Unix time does not fit in i64")
}

fn print_usage() {
    eprintln!("makise-world verify-package <absolute-manifest-path>");
    eprintln!(
        "makise-world status <absolute-db-path> <absolute-manifest-path> <identity-id> <initial-anchor>"
    );
    eprintln!(
        "makise-world serve <absolute-socket-path> <absolute-db-path> <absolute-manifest-path> <identity-id> <initial-anchor>"
    );
}
