"""Errors raised by the Quillon ingest endpoint.

SYNTHETIC FIXTURE — invented for benchmark use.
"""


class ValidationError(ValueError):
    """A record did not satisfy the schema it was validated against.

    Raised by `Ingestor.ingest`. Callers upstream translate this into a 4xx
    for the sending client, so raising it is a customer-visible rejection.
    """
