// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

import SwiftUI

/// L2 — the two-pane shell. Sidebar lists experiments (grouped by tenant);
/// detail renders the selected experiment's matrix. Selection in the sidebar
/// drives the matrix fetch.
struct ContentView: View {
    @Environment(AppStore.self) private var store

    var body: some View {
        NavigationSplitView {
            ExperimentSidebar()
                .navigationTitle(Strings.appTitle)
                .frame(minWidth: Tokens.Sidebar.minWidth)
        } detail: {
            detail
                .toolbar {
                    ToolbarItem(placement: .primaryAction) {
                        Button(Strings.refresh, systemImage: "arrow.clockwise") {
                            Task { await store.loadExperiments() }
                        }
                    }
                }
        }
        .task { await store.loadExperiments() }
        .onChange(of: store.selectedId) { _, id in
            guard let id else { return }
            Task { await store.loadMatrix(for: id) }
        }
    }

    @ViewBuilder
    private var detail: some View {
        switch store.matrixPhase {
        case .empty:
            ContentUnavailableView(Strings.selectExperiment, systemImage: "square.grid.3x3")
        case .loading:
            ProgressView(Strings.loading)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        case .failed(let message):
            ContentUnavailableView(
                Strings.errorTitle,
                systemImage: "bolt.horizontal.circle",
                description: Text(message)
            )
        case .loaded(let matrix):
            MatrixView(matrix: matrix)
        }
    }
}
