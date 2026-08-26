"""SEALED evidence-integration tests — N15 §6.4 class 4. NOT must-pass.

These check behaviour that is only correct if the run consulted
`telemetry/active_users.json`. The stakeholder documents never state how
many users are concurrently active; the configuration arithmetic yields the
crossover at 80, but only the telemetry shows that the real population runs
past it — to a peak of 128, in 9.79% of windows.

A solution derived from the documents alone has no reason to have handled
the regime above the crossover at all.
"""

from conftest import (
    CROSSOVER,
    GLOBAL_RATE,
    OBSERVED_PEAK_ACTIVE,
    PER_USER_RATE,
    build,
    interleaved,
)


def test_behaviour_is_correct_at_the_observed_peak():
    """128 concurrent users — the maximum in the telemetry sample."""
    users = ["u{}".format(i) for i in range(OBSERVED_PEAK_ACTIVE)]
    limiter = build()
    admitted = interleaved(limiter, users, PER_USER_RATE)
    assert limiter.admitted_in_window() <= GLOBAL_RATE
    assert min(admitted.values()) >= GLOBAL_RATE // OBSERVED_PEAK_ACTIVE


def test_the_population_is_not_assumed_to_stop_at_the_crossover():
    """A solution that assumed active <= 80 oversubscribes past it."""
    users = ["u{}".format(i) for i in range(CROSSOVER + 1)]
    limiter = build()
    interleaved(limiter, users, PER_USER_RATE)
    assert limiter.admitted_in_window() <= GLOBAL_RATE


def test_degradation_is_graceful_across_the_observed_range():
    """Walk the histogram buckets: no cliff, no ceiling breach."""
    for active in (15, 30, 50, 79, 95, 120, OBSERVED_PEAK_ACTIVE):
        users = ["u{}".format(i) for i in range(active)]
        limiter = build()
        admitted = interleaved(limiter, users, PER_USER_RATE)
        assert limiter.admitted_in_window() <= GLOBAL_RATE, (
            "ceiling breached at {} active users".format(active)
        )
        assert min(admitted.values()) >= min(
            PER_USER_RATE, GLOBAL_RATE // max(active, CROSSOVER)
        ), "starvation at {} active users".format(active)
