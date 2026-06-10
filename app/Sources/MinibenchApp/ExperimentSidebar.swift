// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

import SwiftUI

/// L2 — the experiments list, grouped by tenant product. The `List`
/// selection is bound to the store; selecting an experiment loads its matrix.
struct ExperimentSidebar: View {
    @Environment(AppStore.self) private var store

    var body: some View {
        @Bindable var store = store
        switch store.experimentsPhase {
        case .loading:
            ProgressView()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        case .failed(let message):
            ContentUnavailableView(
                Strings.errorTitle,
                systemImage: "bolt.horizontal.circle",
                description: Text(message)
            )
        case .loaded(let experiments) where experiments.isEmpty:
            ContentUnavailableView(Strings.noExperiments, systemImage: "tray")
        case .loaded(let experiments):
            List(selection: $store.selectedId) {
                ForEach(grouped(experiments)) { group in
                    Section(group.product.capitalized) {
                        ForEach(group.experiments) { experiment in
                            row(experiment).tag(experiment.id)
                        }
                    }
                }
            }
        }
    }

    private func row(_ experiment: Experiment) -> some View {
        VStack(alignment: .leading, spacing: Tokens.Spacing.tight) {
            Text(experiment.experimentId)
            Text("\(experiment.variants.count) \(Strings.variantsSuffix)")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }

    /// Stable, first-seen grouping of experiments by tenant product.
    private func grouped(_ experiments: [Experiment]) -> [ProductGroup] {
        var order: [String] = []
        var byProduct: [String: [Experiment]] = [:]
        for experiment in experiments {
            if byProduct[experiment.product] == nil { order.append(experiment.product) }
            byProduct[experiment.product, default: []].append(experiment)
        }
        return order.map { ProductGroup(product: $0, experiments: byProduct[$0] ?? []) }
    }
}

/// A sidebar section — the experiments under one tenant product.
private struct ProductGroup: Identifiable {
    let product: String
    let experiments: [Experiment]

    var id: String { product }
}
