// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

import Foundation

/// Swift mirror of the kernel's `ComparisonMatrix` (workbench-contract /
/// minibench-kernel). Decoded from `GET /v1/comparison`. snake_case wire
/// keys are mapped by the decoder's `.convertFromSnakeCase` strategy, so
/// no `CodingKeys` are needed for the field names.
struct ComparisonMatrix: Decodable {
    let experimentId: String
    let variants: [String]
    let variantReplicates: [Int]
    let rows: [ComparisonRow]
    let warnings: [String]
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
    let withinScoreSpread: Double
    let confidenceMin: Double
    let confidenceMax: Double
    let confidenceSpread: Double
    let spreadSignal: SpreadSignal
    let statusDivergence: Bool
    let coverageDivergence: Bool
    let statusUnstable: Bool

    var id: String { stateVarId }

    private enum CodingKeys: String, CodingKey {
        case stateVarId
        case cells
        case scoreSpread
        case withinScoreSpread
        case confidenceMin
        case confidenceMax
        case confidenceSpread
        case spreadSignal
        case statusDivergence
        case coverageDivergence
        case statusUnstable
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        stateVarId = try container.decode(String.self, forKey: .stateVarId)
        cells = try container.decode([ComparisonCell?].self, forKey: .cells)
        scoreSpread = try container.decode(Double.self, forKey: .scoreSpread)
        withinScoreSpread = try container.decode(Double.self, forKey: .withinScoreSpread)
        confidenceMin = try container.decodeIfPresent(Double.self, forKey: .confidenceMin) ?? 0
        confidenceMax = try container.decodeIfPresent(Double.self, forKey: .confidenceMax) ?? 0
        confidenceSpread = try container.decodeIfPresent(Double.self, forKey: .confidenceSpread) ?? 0
        spreadSignal = try container.decodeIfPresent(SpreadSignal.self, forKey: .spreadSignal)
            ?? .noSpread
        statusDivergence = try container.decode(Bool.self, forKey: .statusDivergence)
        coverageDivergence = try container.decode(Bool.self, forKey: .coverageDivergence)
        statusUnstable = try container.decode(Bool.self, forKey: .statusUnstable)
    }
}

/// One variant's evaluation of a state variable.
struct ComparisonCell: Decodable {
    let status: StateStatus
    let score: Double
    let scoreMin: Double
    let scoreMax: Double
    let scoreSd: Double
    let confidence: Double
    let confidenceMin: Double
    let confidenceMax: Double
    let presentCount: Int
    let replicateCount: Int
    let statusUnstable: Bool
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

enum SpreadSignal: String, Decodable {
    case noSpread = "no_spread"
    case singleRun = "single_run"
    case betweenExceedsWithin = "between_exceeds_within"
    case withinMatchesBetween = "within_matches_between"
}
