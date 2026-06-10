// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

import Foundation
import Observation

/// L1 application state — drives both panes: the experiment list (sidebar)
/// and the selected experiment's matrix (detail). Owned at the app root;
/// `@MainActor` because every mutation drives SwiftUI.
@MainActor
@Observable
final class AppStore {
    /// Sidebar state.
    enum ExperimentsPhase {
        case loading
        case loaded([Experiment])
        case failed(String)
    }

    /// Detail (matrix) state.
    enum MatrixPhase {
        case empty
        case loading
        case loaded(ComparisonMatrix)
        case failed(String)
    }

    private(set) var experimentsPhase: ExperimentsPhase = .loading
    /// Bound to the sidebar `List` selection.
    var selectedId: String?
    private(set) var matrixPhase: MatrixPhase = .empty

    /// Fetch the experiment list; auto-select the first and load its matrix.
    func loadExperiments() async {
        experimentsPhase = .loading
        do {
            let experiments: [Experiment] = try await fetch(Routes.experiments)
            experimentsPhase = .loaded(experiments)
            if selectedId == nil || !experiments.contains(where: { $0.id == selectedId }) {
                selectedId = experiments.first?.id
            }
            if let id = selectedId {
                await loadMatrix(for: id)
            } else {
                matrixPhase = .empty
            }
        } catch let error as LoadError {
            experimentsPhase = .failed(error.message)
        } catch {
            experimentsPhase = .failed(Strings.errorUnreachable)
        }
    }

    /// Fetch the comparison matrix for one experiment.
    func loadMatrix(for experimentId: String) async {
        matrixPhase = .loading
        do {
            matrixPhase = .loaded(try await fetch(Routes.matrix(experimentId)))
        } catch let error as LoadError {
            matrixPhase = .failed(error.message)
        } catch {
            matrixPhase = .failed(Strings.errorUnreachable)
        }
    }

    /// GET + decode a data-plane route, mapping each failure stage to a
    /// distinct operator-facing message (this is a boundary, so failures are
    /// surfaced as values, not thrown past it).
    private func fetch<T: Decodable>(_ path: String) async throws -> T {
        guard let url = URL(string: Routes.base + path) else {
            throw LoadError(Strings.errorBadEndpoint)
        }
        let data: Data
        let response: URLResponse
        do {
            (data, response) = try await URLSession.shared.data(from: url)
        } catch {
            throw LoadError(Strings.errorUnreachable)
        }
        guard let http = response as? HTTPURLResponse, http.statusCode == Self.httpOK else {
            throw LoadError(Strings.errorBadStatus)
        }
        do {
            let decoder = JSONDecoder()
            decoder.keyDecodingStrategy = .convertFromSnakeCase
            return try decoder.decode(T.self, from: data)
        } catch {
            throw LoadError(Strings.errorBadPayload)
        }
    }

    /// HTTP 200 — the only success status the data-plane routes return.
    private static let httpOK = 200
}

/// A boundary failure carrying the operator-facing message to show.
private struct LoadError: Error {
    let message: String
    init(_ message: String) { self.message = message }
}
