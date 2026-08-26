"""SEALED acceptance-suite helpers. Never mounted in a participant capsule."""

import pytest

from src.config import parse_limits
from src.limiter import Limiter

GLOBAL_RATE = 800
PER_USER_RATE = 10
CROSSOVER = GLOBAL_RATE // PER_USER_RATE  # 80 active users
OBSERVED_PEAK_ACTIVE = 128  # telemetry/active_users.json summary.max


def build(**overrides):
    """A limiter under the shipped two-key configuration unless overridden."""
    config = {"global_rate": GLOBAL_RATE, "per_user_rate": PER_USER_RATE}
    config.update(overrides)
    return Limiter(config)


def interleaved(limiter, users, rounds, now=0.0):
    """Send `rounds` requests per user, round-robin.

    Requests from concurrently active users interleave within a one-second
    window; sending each user's whole burst before the next would make the
    result a function of arrival order rather than of the admission policy.
    """
    admitted = {user: 0 for user in users}
    for _ in range(rounds):
        for user in users:
            if limiter.allow(user, now):
                admitted[user] += 1
    return admitted


@pytest.fixture
def two_key_config_text():
    return "global_rate: {}\nper_user_rate: {}\n".format(GLOBAL_RATE, PER_USER_RATE)


@pytest.fixture
def parsed_two_key(two_key_config_text):
    return parse_limits(two_key_config_text)
