// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

import Foundation

/// L0 — the data-plane endpoint, centralized. Loopback (risk :8787, career
/// :8788, minibench :8789).
enum Routes {
    static let base = "http://127.0.0.1:8789"
    static let experiments = "/v1/experiments"

    /// The matrix route for one experiment id, percent-encoded for the path.
    static func matrix(_ experimentId: String) -> String {
        let encoded = experimentId.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed)
            ?? experimentId
        return "/v1/experiments/\(encoded)/matrix"
    }
}
