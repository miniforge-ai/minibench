// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

import SwiftUI

/// L2 view — renders the comparison matrix: state-variable rows × variant
/// columns, with the score spread and a divergence mark. The same matrix
/// the CLI prints, native.
struct MatrixView: View {
    @Environment(MatrixStore.self) private var store

    var body: some View {
        content
            .padding(Tokens.Padding.pane)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    Button(Strings.refresh, systemImage: "arrow.clockwise") {
                        Task { await store.load() }
                    }
                }
            }
    }

    @ViewBuilder
    private var content: some View {
        switch store.phase {
        case .idle, .loading:
            ProgressView(Strings.loading)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        case .failed(let message):
            ContentUnavailableView(
                Strings.errorTitle,
                systemImage: "bolt.horizontal.circle",
                description: Text(message)
            )
        case .loaded(let matrix):
            matrixGrid(matrix)
        }
    }

    private func matrixGrid(_ matrix: ComparisonMatrix) -> some View {
        VStack(alignment: .leading, spacing: Tokens.Spacing.normal) {
            Text(matrix.experimentId)
                .font(.title3.weight(.semibold))

            Grid(
                alignment: .leadingFirstTextBaseline,
                horizontalSpacing: Tokens.Spacing.roomy,
                verticalSpacing: Tokens.Spacing.normal
            ) {
                GridRow {
                    Text(Strings.colStateVar)
                    ForEach(matrix.variants, id: \.self) { Text($0) }
                    Text(Strings.colSpread)
                }
                .font(.caption.weight(.semibold))
                .foregroundStyle(.secondary)

                Divider()

                ForEach(matrix.rows) { row in
                    GridRow {
                        Text(row.stateVarId)
                            .font(.system(.body, design: .monospaced))
                        ForEach(Array(row.cells.enumerated()), id: \.offset) { item in
                            cell(item.element)
                        }
                        spread(row)
                    }
                }
            }
        }
    }

    @ViewBuilder
    private func cell(_ cell: ComparisonCell?) -> some View {
        if let cell {
            HStack(spacing: Tokens.Spacing.tight) {
                Text(cell.status.label)
                    .foregroundStyle(cell.status.tint)
                    .fontWeight(.medium)
                Text(cell.score, format: .number.precision(.fractionLength(2)))
                    .foregroundStyle(.secondary)
                    .monospacedDigit()
            }
        } else {
            Text(Strings.absentCell)
                .foregroundStyle(.tertiary)
        }
    }

    private func spread(_ row: ComparisonRow) -> some View {
        HStack(spacing: Tokens.Spacing.tight) {
            Text(row.scoreSpread, format: .number.precision(.fractionLength(2)))
                .monospacedDigit()
            if row.statusDivergence {
                Text(Strings.divergeMark)
                    .foregroundStyle(.orange)
            }
        }
    }
}
