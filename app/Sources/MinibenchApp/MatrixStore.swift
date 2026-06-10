// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

import Foundation
import Observation

/// L1 application state — fetches the comparison matrix from the running
/// data-plane and exposes a single `phase` the view renders. Owned at the
/// app root; `@MainActor` because every mutation drives SwiftUI.
@MainActor
@Observable
final class MatrixStore {
    /// The view renders exactly one of these.
    enum Phase {
        case idle
        case loading
        case loaded(ComparisonMatrix)
        case failed(String)
    }

    private(set) var phase: Phase = .idle

    /// Fetch `/v1/comparison` and decode the matrix. Network and decode
    /// failures collapse to a `failed` phase with an operator-facing hint;
    /// this is a boundary (I/O), so the error is surfaced, not thrown.
    func load() async {
        phase = .loading
        guard let endpoint = URL(string: Strings.comparisonEndpoint) else {
            phase = .failed(Strings.errorBadEndpoint)
            return
        }
        // Transport failure (data-plane down / unreachable) — distinct from
        // a decode failure below, so the operator sees the right cause.
        let data: Data
        let response: URLResponse
        do {
            (data, response) = try await URLSession.shared.data(from: endpoint)
        } catch {
            phase = .failed(Strings.errorUnreachable)
            return
        }
        guard let http = response as? HTTPURLResponse, http.statusCode == Self.httpOK else {
            phase = .failed(Strings.errorBadStatus)
            return
        }
        // Reachable + 200, but the payload shape didn't decode.
        do {
            let decoder = JSONDecoder()
            decoder.keyDecodingStrategy = .convertFromSnakeCase
            phase = .loaded(try decoder.decode(ComparisonMatrix.self, from: data))
        } catch {
            phase = .failed(Strings.errorBadPayload)
        }
    }

    /// HTTP 200 — the only status the comparison route returns on success.
    private static let httpOK = 200
}
