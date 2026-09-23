"""Virus scanning for client uploads, via a ClamAV daemon (clamd).

Spec §15.12: uploads are untrusted until scanned clean, hash-verified, and
ingested -- and `server/src/main.rs::pull` already only offers the desktop
uploads where `scan_status = 'Clean'`. Before this module, nothing ever set
that column to anything but its default, so no client upload could ever
reach an attorney no matter how long it sat in the queue. This is the other
half of that pipe.

PROTOCOL

Speaks clamd's INSTREAM command directly over TCP (the same protocol
`clamdscan --stream` uses): a null-terminated `zINSTREAM` command, the
payload as a sequence of four-byte-length-prefixed chunks terminated by a
zero-length chunk, then a single null-terminated verdict line such as
`stream: OK` or `stream: Eicar-Test-Signature FOUND`.

WHAT IS NOT HERE

A live clamd. `clamd_host` is unset until one is deployed alongside the
portal API; until then `scan_bytes` returns None -- "not attempted", not
"failed" -- so an upload is correctly left at its default scan_status of
Pending rather than incorrectly marked Failed. See app/object_store.py for
the same split applied to the quarantine bucket.
"""

import asyncio
import logging
import struct

from app.config import get_settings

log = logging.getLogger(__name__)

CLEAN = "Clean"
INFECTED = "Infected"
FAILED = "Failed"

_CHUNK_SIZE = 8192


async def scan_bytes(data: bytes) -> str | None:
    """CLEAN, INFECTED or FAILED once a scan was attempted. None if no
    scanner is configured -- the caller should leave scan_status untouched
    rather than write a verdict that was never actually reached."""
    settings = get_settings()
    if settings.clamd_host is None:
        log.info("clamd not configured; upload left unscanned")
        return None

    try:
        reader, writer = await asyncio.wait_for(
            asyncio.open_connection(settings.clamd_host, settings.clamd_port),
            timeout=settings.clamd_timeout_seconds,
        )
    except (OSError, TimeoutError) as e:
        log.error("clamd connection failed: %s", e)
        return FAILED

    try:
        writer.write(b"zINSTREAM\0")
        for offset in range(0, len(data), _CHUNK_SIZE):
            chunk = data[offset : offset + _CHUNK_SIZE]
            writer.write(struct.pack("!L", len(chunk)) + chunk)
        writer.write(struct.pack("!L", 0))  # zero-length chunk ends the stream
        await asyncio.wait_for(writer.drain(), timeout=settings.clamd_timeout_seconds)

        raw = await asyncio.wait_for(
            reader.readuntil(b"\0"), timeout=settings.clamd_timeout_seconds
        )
    except (OSError, TimeoutError, asyncio.IncompleteReadError, asyncio.LimitOverrunError) as e:
        log.error("clamd scan failed: %s", e)
        return FAILED
    finally:
        writer.close()
        try:
            await writer.wait_closed()
        except OSError:
            pass

    verdict = raw.rstrip(b"\0").decode(errors="replace").strip()

    if verdict.endswith("OK"):
        return CLEAN
    if "FOUND" in verdict:
        log.warning("clamd flagged an upload: %s", verdict)
        return INFECTED

    log.error("clamd returned an unexpected verdict: %s", verdict)
    return FAILED
