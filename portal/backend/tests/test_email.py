"""app/email.py against a real (fake) SMTP server.

The fake server speaks just enough of RFC 5321 for `smtplib` to complete a
send: EHLO, MAIL FROM, RCPT TO, DATA, AUTH. That is enough to prove the wire
protocol, not just the message object, without needing TLS certificates or a
live relay.
"""

import asyncio
import socket
import threading

import pytest

from app.config import get_settings
from app.email import send_otp_email


class _FakeSMTPServer:
    """Accepts exactly one connection and records what it was sent."""

    def __init__(self):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.bind(("127.0.0.1", 0))
        self.sock.listen(1)
        self.host, self.port = self.sock.getsockname()
        self.received: list[dict] = []
        self._thread = threading.Thread(target=self._serve_one, daemon=True)
        self._thread.start()

    def _serve_one(self):
        try:
            conn, _ = self.sock.accept()
        except OSError:
            return
        with conn:
            f = conn.makefile("rwb")

            def send(line: bytes):
                f.write(line + b"\r\n")
                f.flush()

            send(b"220 fake.smtp.test ESMTP")
            mail_from = rcpt_to = None
            data_lines: list[bytes] = []
            in_data = False
            while True:
                line = f.readline()
                if not line:
                    break
                if in_data:
                    if line.rstrip(b"\r\n") == b".":
                        in_data = False
                        self.received.append(
                            {
                                "mail_from": mail_from,
                                "rcpt_to": rcpt_to,
                                "data": b"".join(data_lines),
                            }
                        )
                        send(b"250 OK: queued")
                        continue
                    data_lines.append(line)
                    continue

                cmd = line.strip().upper()
                if cmd.startswith(b"EHLO") or cmd.startswith(b"HELO"):
                    send(b"250-fake.smtp.test greets you")
                    send(b"250 AUTH PLAIN LOGIN")
                elif cmd.startswith(b"MAIL FROM"):
                    mail_from = line.strip()
                    send(b"250 OK")
                elif cmd.startswith(b"RCPT TO"):
                    rcpt_to = line.strip()
                    send(b"250 OK")
                elif cmd.startswith(b"AUTH"):
                    send(b"235 Authentication successful")
                elif cmd == b"DATA":
                    in_data = True
                    data_lines = []
                    send(b"354 End data with <CR><LF>.<CR><LF>")
                elif cmd == b"QUIT":
                    send(b"221 Bye")
                    break
                else:
                    send(b"500 unrecognized command")
        self.sock.close()

    def join(self, timeout: float = 5.0):
        self._thread.join(timeout)


def _configure_smtp(monkeypatch, **overrides):
    env = {
        "PORTAL_SMTP_HOST": overrides.get("host", "127.0.0.1"),
        "PORTAL_SMTP_PORT": str(overrides.get("port", 25)),
        "PORTAL_SMTP_USE_TLS": "false",
        "PORTAL_SMTP_FROM_ADDRESS": "Persist <no-reply@persistas.test>",
    }
    if "username" in overrides:
        env["PORTAL_SMTP_USERNAME"] = overrides["username"]
        env["PORTAL_SMTP_PASSWORD"] = overrides.get("password", "secret")
    for key, value in env.items():
        monkeypatch.setenv(key, value)
    get_settings.cache_clear()


@pytest.fixture(autouse=True)
def _restore_settings():
    yield
    get_settings.cache_clear()


async def test_unconfigured_smtp_returns_false_without_raising(monkeypatch):
    monkeypatch.delenv("PORTAL_SMTP_HOST", raising=False)
    get_settings.cache_clear()

    delivered = await send_otp_email("anita@client-a.test", "123456")

    assert delivered is False


async def test_delivers_a_real_message_over_the_wire(monkeypatch):
    server = _FakeSMTPServer()
    _configure_smtp(monkeypatch, port=server.port)

    delivered = await send_otp_email("anita@client-a.test", "482913")
    server.join()

    assert delivered is True
    assert len(server.received) == 1
    sent = server.received[0]
    assert b"anita@client-a.test" in sent["rcpt_to"]
    assert b"no-reply@persistas.test" in sent["mail_from"]
    body = sent["data"].decode()
    assert "482913" in body
    assert "10 minutes" in body  # default otp_ttl_minutes


async def test_authenticates_when_credentials_are_configured(monkeypatch):
    server = _FakeSMTPServer()
    _configure_smtp(monkeypatch, port=server.port, username="relay-user", password="relay-pass")

    delivered = await send_otp_email("anita@client-a.test", "111111")
    server.join()

    assert delivered is True
    assert len(server.received) == 1


async def test_unreachable_relay_returns_false_without_raising(monkeypatch):
    # A closed listening socket on loopback: connection refused, fast and
    # deterministic, no timeout to wait out.
    probe = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    probe.bind(("127.0.0.1", 0))
    _, dead_port = probe.getsockname()
    probe.close()

    _configure_smtp(monkeypatch, port=dead_port)

    delivered = await send_otp_email("anita@client-a.test", "111111")

    assert delivered is False


async def test_a_negative_control_a_broken_server_is_not_reported_as_delivered(monkeypatch):
    """If this test can't fail, `send_otp_email` isn't actually checking
    anything — it would report success regardless of what the relay does."""
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.bind(("127.0.0.1", 0))
    server.listen(1)
    _, port = server.getsockname()

    def reject():
        conn, _ = server.accept()
        with conn:
            conn.sendall(b"554 no thanks\r\n")
        server.close()

    threading.Thread(target=reject, daemon=True).start()
    _configure_smtp(monkeypatch, port=port)

    delivered = await send_otp_email("anita@client-a.test", "111111")

    assert delivered is False
