// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

import Foundation

/// L0 — the data-plane endpoint, centralized. Loopback (risk :8787, career
/// :8788, minibench :8789).
enum Routes {
    static let base = "http://127.0.0.1:8789"
    static let experiments = "/v1/experiments"

    /// The matrix route for one experiment id, encoded as a single path
    /// *component* — `/` is excluded so an id containing a slash can't be
    /// read as a path separator and miss the `:id` segment.
    static func matrix(_ experimentId: String) -> String {
        var allowed = CharacterSet.urlPathAllowed
        allowed.remove("/")
        let encoded = experimentId.addingPercentEncoding(withAllowedCharacters: allowed)
            ?? experimentId
        return "/v1/experiments/\(encoded)/matrix"
    }
}
