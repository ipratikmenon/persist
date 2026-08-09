"""Rate limiting.

Per spec §10: request-otp 3/email/hour and 10/IP/hour, verify-otp 10/IP/hour.

In-memory storage, which is honest for a single uvicorn worker and wrong for
several — the limits become per-worker. That is recorded rather than papered
over; a Redis backend is the fix when the portal runs more than one process.
"""

from slowapi import Limiter
from slowapi.util import get_remote_address

limiter = Limiter(key_func=get_remote_address, headers_enabled=True)
