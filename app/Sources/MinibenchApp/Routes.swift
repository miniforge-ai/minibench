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

import Foundation

/// L0 — the data-plane endpoint, centralized. Loopback (risk :8787, career
/// :8788, minibench :8789).
enum Routes {
    /// Data-plane base URL. `MINIBENCH_DATA_PLANE_URL` overrides the loopback
    /// default per `foundations/config-as-data` — operational value out of
    /// code, the literal is only the fallback.
    static let base = ProcessInfo.processInfo.environment["MINIBENCH_DATA_PLANE_URL"]
        ?? "http://127.0.0.1:8789"
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
