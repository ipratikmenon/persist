"""Database connections.

Three engines, three PostgreSQL roles. The separation is the point: the
connection that serves a client's matters physically cannot write, and the
connection that logs them in physically cannot read a matter.

THE RLS CONTRACT

`client_scope` opens a transaction and issues:

    SET LOCAL app.current_client_id = '<cid from the JWT>'

`SET LOCAL` is scoped to the transaction, so a pooled connection cannot carry
one client's context into the next request — which is the failure mode that
makes connection pooling and row-level security dangerous together.

Every read of `mirror.*` goes through `client_scope`. There is no other way to
get a reader connection, deliberately: a helper that returned a bare connection
would be one `await` away from an unscoped query.
"""

from collections.abc import AsyncIterator
from contextlib import asynccontextmanager

from sqlalchemy import text
from sqlalchemy.ext.asyncio import AsyncConnection, AsyncEngine, create_async_engine

from app.config import get_settings

_engines: dict[str, AsyncEngine] = {}


def _engine(name: str, dsn: str) -> AsyncEngine:
    if name not in _engines:
        _engines[name] = create_async_engine(
            dsn,
            pool_size=5,
            max_overflow=5,
            pool_pre_ping=True,
            # No statement cache: asyncpg's prepared statements are per
            # connection, and RLS makes the same SQL return different rows.
            connect_args={"statement_cache_size": 0},
        )
    return _engines[name]


def reader_engine() -> AsyncEngine:
    return _engine("reader", get_settings().reader_dsn)


def writer_engine() -> AsyncEngine:
    return _engine("writer", get_settings().writer_dsn)


def auth_engine() -> AsyncEngine:
    return _engine("auth", get_settings().auth_dsn)


async def dispose_engines() -> None:
    for engine in _engines.values():
        await engine.dispose()
    _engines.clear()


@asynccontextmanager
async def client_scope(client_id: str) -> AsyncIterator[AsyncConnection]:
    """A read connection scoped to one client for the life of one transaction.

    The client id is bound as a parameter, never interpolated: it arrives from a
    signed JWT, but `set_config` with a bind parameter means even a forged claim
    can only ever be a client id, not SQL.
    """
    async with reader_engine().begin() as conn:
        await conn.execute(
            text("SELECT set_config('app.current_client_id', :cid, true)"),
            {"cid": client_id},
        )
        yield conn


@asynccontextmanager
async def client_write_scope(client_id: str) -> AsyncIterator[AsyncConnection]:
    """The same scoping for the two inbound tables a client may write to."""
    async with writer_engine().begin() as conn:
        await conn.execute(
            text("SELECT set_config('app.current_client_id', :cid, true)"),
            {"cid": client_id},
        )
        yield conn


@asynccontextmanager
async def auth_scope() -> AsyncIterator[AsyncConnection]:
    """The login path. No client id exists yet — establishing one is the point."""
    async with auth_engine().begin() as conn:
        yield conn
