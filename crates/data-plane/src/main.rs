// Title: Minibench
// Subtitle: data-plane binary — bind loopback and serve workbench snapshots
// Author: Christopher Lester
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

//! Loads snapshots from `MINIBENCH_SNAPSHOT_DIR` (default `fixtures`)
//! and serves them on loopback `:8789` (after risk `:8787`, career
//! `:8788`) via the shared foundation router.

use std::net::SocketAddr;

use minibench_data_plane::{WorkbenchProvider, router, strings};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::var("MINIBENCH_SNAPSHOT_DIR").unwrap_or_else(|_| "fixtures".to_string());
    let provider = WorkbenchProvider::from_dir(&dir)?;
    let app = router(provider);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8789));
    println!(
        "{} http://{addr} ({} {dir})",
        strings::BANNER_PREFIX,
        strings::BANNER_SNAPSHOTS_FROM
    );
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
