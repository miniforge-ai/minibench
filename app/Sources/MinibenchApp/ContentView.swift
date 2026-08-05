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
                        .keyboardShortcut("r", modifiers: .command)
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
            ContentUnavailableView {
                Label(Strings.errorTitle, systemImage: "bolt.horizontal.circle")
            } description: {
                Text(message)
            } actions: {
                Button(Strings.retry) { Task { await store.loadExperiments() } }
            }
        case .loaded(let matrix):
            MatrixView(matrix: matrix)
        }
    }
}
