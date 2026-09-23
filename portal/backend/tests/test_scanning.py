"""app/scanning.py against a fake clamd -- the real INSTREAM wire protocol,
just not a real virus engine behind it.
"""

import asyncio
import socket
import struct
import threading

import pytest

from app.config import get_settings
from app.scanning import CLEAN, FAILED, INFECTED, scan_bytes

# The standard antivirus test string. Not a virus; every scanner (including
# real ClamAV) is built to flag it by name, which is the whole point of it
# existing, so a fake scanner that flags exactly this string is behaving
# like a real one would.
EICAR = (
    b"X5O!P%@AP[4\\PZX54(P^)7CC)7}$EICAR-STANDARD-ANTIVIRUS-TEST-FILE!$H+H*"
)


class _FakeClamd:
    """Reads a zINSTREAM session to completion, then replies with a fixed
    verdict -- proving our client speaks the real chunked framing, not
    merely that it can parse a canned response."""

    def __init__(self, verdict: bytes, *, drop_connection: bool = False):
        self.verdict = verdict
        self.drop_connection = drop_connection
        self.received = b""
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.bind(("127.0.0.1", 0))
        self.sock.listen(1)
        self.host, self.port = self.sock.getsockname()
        self._thread = threading.Thread(target=self._serve_one, daemon=True)
        self._thread.start()

    def _read_exact(self, conn: socket.socket, n: int) -> bytes:
        buf = b""
        while len(buf) < n:
            chunk = conn.recv(n - len(buf))
            if not chunk:
                raise ConnectionError("client closed early")
            buf += chunk
        return buf

    def _serve_one(self):
        try:
            conn, _ = self.sock.accept()
        except OSError:
            return
        with conn:
            try:
                command = self._read_exact(conn, len(b"zINSTREAM\0"))
                assert command == b"zINSTREAM\0"

                payload = bytearray()
                while True:
                    length_bytes = self._read_exact(conn, 4)
                    (length,) = struct.unpack("!L", length_bytes)
                    if length == 0:
                        break
                    payload += self._read_exact(conn, length)
                self.received = bytes(payload)

                if self.drop_connection:
                    return
                conn.sendall(self.verdict)
            except (ConnectionError, OSError):
                pass
        self.sock.close()

    def join(self, timeout: float = 5.0):
        self._thread.join(timeout)


def _configure_clamd(monkeypatch, port: int, **overrides):
    monkeypatch.setenv("PORTAL_CLAMD_HOST", "127.0.0.1")
    monkeypatch.setenv("PORTAL_CLAMD_PORT", str(port))
    monkeypatch.setenv("PORTAL_CLAMD_TIMEOUT_SECONDS", str(overrides.get("timeout", 5.0)))
    get_settings.cache_clear()


@pytest.fixture(autouse=True)
def _restore_settings():
    yield
    get_settings.cache_clear()


async def test_unconfigured_clamd_returns_none_not_failed(monkeypatch):
    """None means 'not attempted' -- distinct from FAILED, so the caller
    leaves scan_status at its Pending default instead of writing a verdict
    that was never actually reached."""
    monkeypatch.delenv("PORTAL_CLAMD_HOST", raising=False)
    get_settings.cache_clear()

    result = await scan_bytes(b"whatever")

    assert result is None


async def test_a_clean_file_is_reported_clean(monkeypatch):
    server = _FakeClamd(b"stream: OK\0")
    _configure_clamd(monkeypatch, server.port)

    result = await scan_bytes(b"%PDF-1.4 an ordinary document")
    server.join()

    assert result == CLEAN
    assert server.received == b"%PDF-1.4 an ordinary document"


async def test_the_eicar_string_is_reported_infected(monkeypatch):
    server = _FakeClamd(b"stream: Eicar-Test-Signature FOUND\0")
    _configure_clamd(monkeypatch, server.port)

    result = await scan_bytes(EICAR)
    server.join()

    assert result == INFECTED
    assert server.received == EICAR


async def test_a_file_larger_than_one_chunk_streams_correctly(monkeypatch):
    """Exercises the chunk-splitting loop, not just a single small write."""
    server = _FakeClamd(b"stream: OK\0")
    _configure_clamd(monkeypatch, server.port)

    payload = b"A" * (8192 * 3 + 17)  # not a whole multiple of the chunk size
    result = await scan_bytes(payload)
    server.join()

    assert result == CLEAN
    assert server.received == payload


async def test_an_unreachable_daemon_is_reported_failed_not_clean(monkeypatch):
    probe = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    probe.bind(("127.0.0.1", 0))
    _, dead_port = probe.getsockname()
    probe.close()

    _configure_clamd(monkeypatch, dead_port)

    result = await scan_bytes(b"whatever")

    assert result == FAILED


async def test_a_connection_dropped_mid_scan_is_reported_failed_not_clean(monkeypatch):
    server = _FakeClamd(b"", drop_connection=True)
    _configure_clamd(monkeypatch, server.port)

    result = await scan_bytes(b"whatever")
    server.join()

    assert result == FAILED


async def test_negative_control_a_verdict_containing_found_must_not_pass_as_clean(monkeypatch):
    """If this can't fail, an infected verdict could slip through as Clean."""
    server = _FakeClamd(b"stream: Some-Malware-Signature FOUND\0")
    _configure_clamd(monkeypatch, server.port)

    result = await scan_bytes(b"whatever")
    server.join()

    assert result != CLEAN
