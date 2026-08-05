// Title: Minibench
// Subtitle: data-plane binary — bind loopback and serve workbench snapshots
// Author: Christopher Lester
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
