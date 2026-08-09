"""The portal API.

Every route past /auth requires a verified JWT and runs inside a transaction
scoped by `SET LOCAL app.current_client_id`. A client cannot reach another
client's row through this app: row-level security refuses it at the database,
every query filters by client_id as a second lock, and every response passes
through a Pydantic model that does not carry internal columns.
"""

import logging
from collections.abc import AsyncIterator
from contextlib import asynccontextmanager

from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from slowapi import _rate_limit_exceeded_handler
from slowapi.errors import RateLimitExceeded

from app.api import auth, documents, invoices, matters, misc
from app.config import get_settings
from app.db.session import dispose_engines
from app.rate_limit import limiter

log = logging.getLogger(__name__)


@asynccontextmanager
async def lifespan(app: FastAPI) -> AsyncIterator[None]:
    settings = get_settings()
    # Read the keys at startup rather than on the first login: a missing or
    # unreadable key should stop the process, not fail one client's request.
    _ = settings.jwt_private_key, settings.jwt_public_key
    log.info("portal API starting")
    yield
    await dispose_engines()


app = FastAPI(
    title="Persist Client Portal",
    version="0.1.0",
    lifespan=lifespan,
    # The client portal is not a public API. The schema would only describe
    # attack surface to someone who has no business calling it.
    openapi_url=None,
    docs_url=None,
    redoc_url=None,
)

app.state.limiter = limiter
app.add_exception_handler(RateLimitExceeded, _rate_limit_exceeded_handler)

app.add_middleware(
    CORSMiddleware,
    allow_origins=get_settings().allowed_origins,
    allow_credentials=True,  # the refresh cookie
    allow_methods=["GET", "POST", "PATCH", "OPTIONS"],
    allow_headers=["Authorization", "Content-Type"],
)


@app.middleware("http")
async def security_headers(request: Request, call_next):
    """Spec §13. `no-store` is set globally rather than per route — a cache
    directive that has to be remembered on each new endpoint is one that will
    eventually be forgotten on the endpoint that mattered."""
    response = await call_next(request)
    response.headers.setdefault("Strict-Transport-Security", "max-age=31536000; includeSubDomains")
    response.headers.setdefault("X-Content-Type-Options", "nosniff")
    response.headers.setdefault("X-Frame-Options", "DENY")
    response.headers.setdefault("Referrer-Policy", "no-referrer")
    response.headers.setdefault("Cache-Control", "no-store")
    return response


@app.exception_handler(Exception)
async def unhandled(request: Request, exc: Exception) -> JSONResponse:
    """A stack trace or a database message must never reach a client — a failed
    query is quite capable of quoting a column the portal is meant to hide."""
    log.exception("unhandled error on %s %s", request.method, request.url.path)
    return JSONResponse(status_code=500, content={"detail": "Something went wrong."})


app.include_router(auth.router)
app.include_router(matters.router)
app.include_router(documents.router)
app.include_router(invoices.router)
app.include_router(misc.router)
