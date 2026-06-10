// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

import SwiftUI

/// L2 view — renders one comparison matrix: state-variable rows × variant
/// columns, with the score spread and a divergence mark. Pure render of a
/// passed-in matrix; fetch + phase live in the store / ContentView.
struct MatrixView: View {
    let matrix: ComparisonMatrix

    var body: some View {
        VStack(alignment: .leading, spacing: Tokens.Spacing.normal) {
            Text(matrix.experimentId)
                .font(.title3.weight(.semibold))
            grid
        }
        .padding(Tokens.Padding.pane)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    private var grid: some View {
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

            // Span the full grid width: id column + one per variant + spread.
            GridRow {
                Divider().gridCellColumns(matrix.variants.count + 2)
            }

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
