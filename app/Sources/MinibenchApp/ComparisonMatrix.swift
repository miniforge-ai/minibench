// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

import Foundation

/// Swift mirror of the kernel's `ComparisonMatrix` (workbench-contract /
/// minibench-kernel). Decoded from `GET /v1/comparison`. snake_case wire
/// keys are mapped by the decoder's `.convertFromSnakeCase` strategy, so
/// no `CodingKeys` are needed for the field names.
struct ComparisonMatrix: Decodable {
    let experimentId: String
    let variants: [String]
    let rows: [ComparisonRow]
}

/// Sidebar unit — one experiment present in the loaded snapshots, decoded
/// from `GET /v1/experiments`.
struct Experiment: Decodable, Identifiable, Hashable {
    let experimentId: String
    let product: String
    let variants: [String]

    var id: String { experimentId }
}

/// One state-variable row: a cell per variant (absent where a variant did
/// not evaluate it), the score spread, and whether status diverged.
struct ComparisonRow: Decodable, Identifiable {
    let stateVarId: String
    let cells: [ComparisonCell?]
    let scoreSpread: Double
    let statusDivergence: Bool

    var id: String { stateVarId }
}

/// One variant's evaluation of a state variable.
struct ComparisonCell: Decodable {
    let status: StateStatus
    let score: Double
    let confidence: Double?
}

/// The contract's `StateStatus`. Raw values are the wire strings
/// (snake_case) — key-decoding strategy does not touch scalar values, so
/// these must match the JSON exactly.
enum StateStatus: String, Decodable {
    case pass
    case warn
    case fail
    case blocked
    case notApplicable = "not_applicable"
    case unknown
}
