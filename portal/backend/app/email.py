"""Delivers the OTP code by email.

`otp.request_otp` (app/auth/otp.py) decides whether a code is owed and
generates one; this module only gets it to an inbox once that decision is
made. Splitting the two means the account-enumeration defence (spec §15.11)
never depends on whether delivery actually worked — the route always returns
202 first and attempts delivery after.

Real SMTP, no dev shortcut baked in here: `PORTAL_EXPOSE_OTP_FOR_TESTS` is
what lets tests and manual QA skip this module by reading the code back from
the response instead. The two mechanisms are independent on purpose.

A send failure is logged and swallowed, never surfaced to the browser. The
alternative is an oracle: an attacker watching request-otp could tell "the
mailbox exists but our relay is down" apart from "no such user" by response
shape or timing, which is exactly what the 202-always contract exists to
prevent.
"""

import asyncio
import logging
import smtplib
import ssl
from email.message import EmailMessage

from app.config import get_settings

log = logging.getLogger(__name__)


def _build_message(to: str, code: str, *, from_addr: str, ttl_minutes: int) -> EmailMessage:
    msg = EmailMessage()
    msg["Subject"] = "Your Persist sign-in code"
    msg["From"] = from_addr
    msg["To"] = to
    msg.set_content(
        f"Your Persist sign-in code is {code}.\n\n"
        f"It expires in {ttl_minutes} minutes. If you did not request this, "
        "no action is needed — the code will simply expire."
    )
    return msg


def _send_sync(
    msg: EmailMessage,
    *,
    host: str,
    port: int,
    username: str | None,
    password: str | None,
    use_tls: bool,
    timeout: float,
) -> None:
    """The blocking half. Runs off the event loop — see send_otp_email."""
    # Port 465 is implicit TLS (the handshake happens before any SMTP command);
    # every other port either speaks plaintext or upgrades via STARTTLS.
    smtp_cls = smtplib.SMTP_SSL if (use_tls and port == 465) else smtplib.SMTP
    with smtp_cls(host, port, timeout=timeout) as smtp:
        smtp.ehlo()
        if use_tls and port != 465:
            smtp.starttls(context=ssl.create_default_context())
            smtp.ehlo()
        if username:
            smtp.login(username, password or "")
        smtp.send_message(msg)


async def send_otp_email(to: str, code: str) -> bool:
    """True once handed off to the relay. False if unconfigured or the send
    failed — either way the caller logs and moves on; see module docstring."""
    settings = get_settings()
    if settings.smtp_host is None:
        log.info("smtp not configured; otp code was generated but not emailed")
        return False

    msg = _build_message(
        to,
        code,
        from_addr=settings.smtp_from_address,
        ttl_minutes=settings.otp_ttl_minutes,
    )
    try:
        await asyncio.to_thread(
            _send_sync,
            msg,
            host=settings.smtp_host,
            port=settings.smtp_port,
            username=settings.smtp_username,
            password=settings.smtp_password,
            use_tls=settings.smtp_use_tls,
            timeout=settings.smtp_timeout_seconds,
        )
        return True
    except (OSError, smtplib.SMTPException) as e:
        log.error("otp email delivery to relay failed: %s", e)
        return False
