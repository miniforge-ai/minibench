"""SEALED evidence-integration tests — N15 §6.4 class 4. NOT must-pass.

These check behaviour that is only correct if the run consulted
`telemetry/client_versions.json`. The stakeholder documents say client
releases roll out slowly and that v1 will eventually be retired; they never
say how much v1 traffic there still is, or how fast it is going away.

The telemetry does: v1 is 3.2% of records and falling by 0.15 percentage
points per week, with 41% of that traffic concentrated in two accounts on
multi-year change-control cycles. A run that read it gates retirement of the
pre-region path on the observed share crossing a threshold. A run that did
not will reach for a sunset date or a count of releases, and cannot answer
the question this suite asks.
"""

import inspect

from conftest import (
    OBSERVED_V1_SHARE,
    RESIDUAL_V1_SHARE,
    build,
)

from src.ingest import Ingestor

WALK_DOWN_THE_TREND = (0.0365, 0.0350, 0.0335, OBSERVED_V1_SHARE, 0.02, 0.01, 0.005, 0.001, RESIDUAL_V1_SHARE)


def test_the_predicate_is_driven_by_the_observed_share():
    parameters = list(inspect.signature(Ingestor.can_deprecate_v1).parameters)
    assert parameters[0] == "self"
    assert len(parameters) == 2, (
        "can_deprecate_v1 takes the observed share and nothing else, got {}"
        .format(parameters)
    )


def test_the_observed_share_does_not_permit_deprecation():
    """3.2% of records still arrive from v1 clients."""
    assert build().can_deprecate_v1(OBSERVED_V1_SHARE) is False


def test_a_residual_share_permits_deprecation():
    """0.05% is a tail, not a population. The gate must eventually open."""
    assert build().can_deprecate_v1(RESIDUAL_V1_SHARE) is True


def test_the_predicate_is_monotone_in_the_observed_share():
    """Walking the observed decline down to a residual tail flips the answer
    once and never back."""
    ingestor = build()
    answers = [ingestor.can_deprecate_v1(share) for share in WALK_DOWN_THE_TREND]
    assert answers == sorted(answers), "not monotone: {}".format(answers)
    assert answers[0] is False
    assert answers[-1] is True


def test_the_predicate_does_not_change_between_calls():
    """No hidden state and no clock: the same share gives the same answer."""
    ingestor = build()
    first = ingestor.can_deprecate_v1(OBSERVED_V1_SHARE)
    ingestor.can_deprecate_v1(RESIDUAL_V1_SHARE)
    assert ingestor.can_deprecate_v1(OBSERVED_V1_SHARE) == first


def test_the_gate_does_not_open_anywhere_in_the_observed_sample():
    """Every weekly figure in the 30-day sample is above the threshold.

    A threshold chosen without the telemetry can easily land inside the
    observed range, which retires v1 while it is still carrying traffic.
    """
    ingestor = build()
    for share in (0.0365, 0.0350, 0.0335, 0.0320):
        assert ingestor.can_deprecate_v1(share) is False, (
            "the gate opens at an observed weekly share of {}".format(share)
        )
