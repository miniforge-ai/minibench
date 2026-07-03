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
            warnings
            ScrollView([.horizontal, .vertical]) {
                grid
            }
        }
        .padding(Tokens.Padding.pane)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    @ViewBuilder
    private var warnings: some View {
        if !matrix.warnings.isEmpty {
            VStack(alignment: .leading, spacing: Tokens.Spacing.tight) {
                ForEach(matrix.warnings, id: \.self) { warning in
                    Text(warningLabel(warning))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
        }
    }

    private var grid: some View {
        Grid(
            alignment: .leadingFirstTextBaseline,
            horizontalSpacing: Tokens.Spacing.roomy,
            verticalSpacing: Tokens.Spacing.normal
        ) {
            GridRow {
                Text(Strings.colStateVar)
                ForEach(Array(matrix.variants.enumerated()), id: \.offset) { item in
                    Text(variantLabel(item.offset, item.element))
                }
                Text(Strings.colSpread)
                Text(Strings.colWithin)
                Text(Strings.colSignals)
            }
            .font(.caption.weight(.semibold))
            .foregroundStyle(.secondary)

            // Span the full grid width: id column + variants + metric columns.
            GridRow {
                Divider().gridCellColumns(matrix.variants.count + 4)
            }

            ForEach(matrix.rows) { row in
                GridRow {
                    Text(row.stateVarId)
                        .font(.system(.body, design: .monospaced))
                    ForEach(Array(row.cells.enumerated()), id: \.offset) { item in
                        cell(item.element)
                    }
                    spread(row)
                    Text(row.withinScoreSpread, format: .number.precision(.fractionLength(2)))
                        .monospacedDigit()
                        .foregroundStyle(.secondary)
                    signals(row)
                }
            }
        }
    }

    @ViewBuilder
    private func cell(_ cell: ComparisonCell?) -> some View {
        if let cell {
            VStack(alignment: .leading, spacing: Tokens.Spacing.tight) {
                HStack(spacing: Tokens.Spacing.tight) {
                    Text(cell.status.label)
                        .foregroundStyle(cell.status.tint)
                        .fontWeight(.medium)
                    Text(cell.score, format: .number.precision(.fractionLength(2)))
                        .monospacedDigit()
                    Text(confidenceLabel(cell.confidence))
                        .foregroundStyle(.secondary)
                        .monospacedDigit()
                }
                if cell.replicateCount > 1 {
                    Text(replicateLabel(cell))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .monospacedDigit()
                }
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

    private func signals(_ row: ComparisonRow) -> some View {
        HStack(spacing: Tokens.Spacing.tight) {
            if row.statusDivergence {
                signal(Strings.signalStatus, color: .orange)
            }
            if row.coverageDivergence {
                signal(Strings.signalCoverage, color: .red)
            }
            if row.statusUnstable {
                signal(Strings.signalUnstable, color: .yellow)
            }
        }
    }

    private func signal(_ label: String, color: Color) -> some View {
        Text(label)
            .font(.caption)
            .foregroundStyle(color)
    }

    private func variantLabel(_ index: Int, _ variant: String) -> String {
        guard matrix.variantReplicates.indices.contains(index) else { return variant }
        let replicates = matrix.variantReplicates[index]
        guard replicates > 1 else { return variant }
        return "\(variant) (n=\(replicates))"
    }

    private func confidenceLabel(_ confidence: Double) -> String {
        "\(Strings.confidencePrefix) \(confidence.formatted(.number.precision(.fractionLength(2))))"
    }

    private func replicateLabel(_ cell: ComparisonCell) -> String {
        let min = cell.scoreMin.formatted(.number.precision(.fractionLength(2)))
        let max = cell.scoreMax.formatted(.number.precision(.fractionLength(2)))
        return "\(min)-\(max) \(cell.presentCount)/\(cell.replicateCount)"
    }

    private func warningLabel(_ warning: String) -> String {
        switch warning {
        case "missing_source_hashes":
            Strings.missingSourceHashes
        case "missing_policy_provenance":
            Strings.missingPolicyProvenance
        case "missing_evaluator_provenance":
            Strings.missingEvaluatorProvenance
        default:
            "\(Strings.unknownWarning): \(warning)"
        }
    }
}
