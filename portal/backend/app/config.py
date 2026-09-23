"""Configuration.

Everything here is required. There are no development defaults for the database
URLs or the signing key: a portal that silently starts against the wrong
database, or with a key someone can guess, is worse than one that refuses to
start.
"""

from functools import lru_cache
from pathlib import Path

from pydantic import Field
from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_prefix="PORTAL_", extra="ignore")

    # Three connections, three roles. See server/migrations/0002_rls.sql.
    #   reader — SELECT on mirror.*, every row filtered by RLS
    #   writer — INSERT on the two client-authored inbound tables
    #   auth   — the login path only; cannot read a single matter
    reader_dsn: str
    writer_dsn: str
    auth_dsn: str

    # RS256. The API holds both halves because it issues as well as verifies;
    # only the public half would be needed by a separate verifier.
    jwt_private_key_path: Path
    jwt_public_key_path: Path

    access_token_minutes: int = 15
    refresh_token_days: int = 7

    otp_length: int = 6
    otp_ttl_minutes: int = 10
    otp_max_attempts: int = 5

    signed_url_ttl_seconds: int = 300  # spec §15.5 — 5 minutes, no longer

    # Object storage. Hetzner is not provisioned yet; the signing scheme is
    # real and the base URL is wherever the objects end up being served.
    object_base_url: str
    storage_signing_key: str

    # Upload limits. A client sends a POA or an evidence bundle, not a video.
    max_upload_bytes: int = 25 * 1024 * 1024
    allowed_upload_mime_types: list[str] = Field(
        default_factory=lambda: [
            "application/pdf",
            "image/jpeg",
            "image/png",
            "application/msword",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ]
    )

    # Where the portal is served from, for CORS. A list, because staging exists.
    allowed_origins: list[str] = Field(default_factory=list)

    # Set only in tests. Lets the OTP code be read back instead of emailed.
    expose_otp_for_tests: bool = False

    # SMTP relay for OTP delivery. `smtp_host` unset means "not configured" —
    # the portal still issues codes (so login flow tests and manual QA via
    # expose_otp_for_tests keep working) but does not attempt to send one.
    # A firm deployment sets this; there is no default host, because a wrong
    # default that looks configured is worse than an explicit "not sending".
    smtp_host: str | None = None
    smtp_port: int = 587
    smtp_username: str | None = None
    smtp_password: str | None = None
    smtp_from_address: str = "Persist <no-reply@persistas.example>"
    smtp_use_tls: bool = True
    smtp_timeout_seconds: float = 10.0

    # ClamAV daemon (clamd), TCP INSTREAM protocol. Unset host means "not
    # configured" — uploads are then left at scan_status = 'Pending' rather
    # than waved through, since spec §15.12 requires scanned-clean before the
    # desktop will touch an upload.
    clamd_host: str | None = None
    clamd_port: int = 3310
    clamd_timeout_seconds: float = 15.0

    # Object storage (Hetzner, S3-compatible). Used to write the quarantine
    # copy of a client upload. Required once uploads are enabled in a real
    # deployment; there is no default bucket.
    s3_endpoint_url: str | None = None
    s3_bucket: str | None = None
    s3_region: str = "auto"
    s3_access_key_id: str | None = None
    s3_secret_access_key: str | None = None

    @property
    def jwt_private_key(self) -> str:
        return self.jwt_private_key_path.read_text()

    @property
    def jwt_public_key(self) -> str:
        return self.jwt_public_key_path.read_text()


@lru_cache
def get_settings() -> Settings:
    return Settings()
