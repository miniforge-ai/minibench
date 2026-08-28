"""SEALED resolution tests (must-pass) — N15 §6.4 class 1.

One or more per injected contradiction, encoding the sealed intended
resolution's observable behaviour.
"""

from conftest import (
    CROSSOVER,
    GLOBAL_RATE,
    OBSERVED_PEAK_ACTIVE,
    PER_USER_RATE,
    build,
    interleaved,
)

# --- Contradiction 1: the guarantee cannot hold universally -----------------


def test_published_rate_is_delivered_below_the_crossover():
    """At 40 active users the ceiling has room for everyone's 10 rps."""
    users = ["u{}".format(i) for i in range(40)]
    admitted = interleaved(build(), users, PER_USER_RATE)
    assert all(count == PER_USER_RATE for count in admitted.values())


def test_entitlement_degrades_to_an_equal_share_above_the_crossover():
    """Past the crossover every user still gets the equal share, not zero."""
    users = ["u{}".format(i) for i in range(OBSERVED_PEAK_ACTIVE)]
    limiter = build()
    admitted = interleaved(limiter, users, PER_USER_RATE * 2)
    equal_share = GLOBAL_RATE // OBSERVED_PEAK_ACTIVE
    assert min(admitted.values()) >= equal_share
    assert limiter.admitted_in_window() <= GLOBAL_RATE


def test_the_guarantee_is_not_simply_dropped():
    """Degradation is proportional, not a fallback to global-only shedding."""
    users = ["u{}".format(i) for i in range(100)]
    admitted = interleaved(build(), users, PER_USER_RATE * 2)
    equal_share = GLOBAL_RATE // 100
    assert min(admitted.values()) >= equal_share


def test_a_heavy_sender_cannot_starve_quiet_tenants():
    """Arrival order must not decide who gets served.

    The heavy sender empties its burst before any quiet tenant appears. A
    scheme that lets it borrow the whole allowance leaves nothing behind.
    """
    limiter = build()
    for _ in range(GLOBAL_RATE):
        limiter.allow("heavy", 0.0)
    quiet = ["q{}".format(i) for i in range(10)]
    admitted = interleaved(limiter, quiet, PER_USER_RATE)
    assert all(count == PER_USER_RATE for count in admitted.values())


# --- Contradiction 2: per_user_rate becomes an upper bound ------------------


def test_two_key_configuration_still_produces_correct_behaviour():
    """No new required key: the shipped two-key file must be sufficient."""
    users = ["u{}".format(i) for i in range(OBSERVED_PEAK_ACTIVE)]
    limiter = build()
    admitted = interleaved(limiter, users, PER_USER_RATE * 2)
    assert min(admitted.values()) >= GLOBAL_RATE // OBSERVED_PEAK_ACTIVE
    assert limiter.admitted_in_window() <= GLOBAL_RATE


def test_per_user_rate_is_an_upper_bound_not_a_promise():
    """Under saturation the entitlement is the share, never more."""
    users = ["u{}".format(i) for i in range(OBSERVED_PEAK_ACTIVE)]
    admitted = interleaved(build(), users, PER_USER_RATE * 2)
    assert max(admitted.values()) <= PER_USER_RATE * 2
    assert min(admitted.values()) < PER_USER_RATE


def test_the_crossover_matches_the_configured_arithmetic():
    """Below global_rate // per_user_rate active users, nobody is degraded."""
    users = ["u{}".format(i) for i in range(CROSSOVER)]
    admitted = interleaved(build(), users, PER_USER_RATE)
    assert all(count == PER_USER_RATE for count in admitted.values())
