"""Response models.

Every route returns one of these. No database row is ever serialised directly —
that is how an internal column reaches a client the first time someone adds one
to a SELECT.

The models are also the second lock on the projection. `projection.rs` decides
what leaves the desktop; these decide what leaves the API. A column that somehow
reached the mirror and should not have is still not returned here.
"""

from datetime import date, datetime
from decimal import Decimal

from pydantic import BaseModel, ConfigDict, Field
from pydantic.alias_generators import to_camel


class Base(BaseModel):
    """camelCase on the wire, snake_case in Python.

    Every other surface in Persist is camelCase over the wire — Keel's IPC
    structs carry `#[serde(rename_all = "camelCase")]` and Deck's types match.
    The portal should not be the one place a client-side developer has to
    remember it is different.

    `populate_by_name` keeps the Python-side field names usable when these are
    constructed in code, which is how every route builds them.
    """

    model_config = ConfigDict(
        extra="forbid",
        from_attributes=False,
        alias_generator=to_camel,
        populate_by_name=True,
        serialize_by_alias=True,
    )


# ---------------------------------------------------------------------------
# Matters
# ---------------------------------------------------------------------------


class MatterSummary(Base):
    id: str
    title: str
    matter_type: str
    status: str
    opened_date: date
    next_deadline_date: date | None = None
    next_deadline_event: str | None = None


class DeadlineOut(Base):
    id: str
    matter_id: str
    docketing_event: str
    due_date: date
    status: str


class IpAssetOut(Base):
    id: str
    matter_id: str
    asset_type: str
    title: str
    application_number: str | None = None
    registration_number: str | None = None
    status: str
    classes: list[int] = Field(default_factory=list)
    next_renewal_date: date | None = None


class DocumentOut(Base):
    id: str
    matter_id: str | None = None
    filename: str
    category: str | None = None
    file_size_bytes: int
    shared_at: datetime


class MatterDetail(MatterSummary):
    forum: str | None = None
    jurisdiction: str
    # The client-facing note. `matters.internal_notes` does not exist out here.
    client_notes: str | None = None
    responsible_attorney: str | None = None
    deadlines: list[DeadlineOut] = Field(default_factory=list)
    ip_assets: list[IpAssetOut] = Field(default_factory=list)
    documents: list[DocumentOut] = Field(default_factory=list)


# ---------------------------------------------------------------------------
# Invoices
#
# Deliberately absent: line items. `time_entries` never syncs, so the HTML view
# shows totals and the GST breakdown; the PDF is the authoritative document
# (spec §14).
# ---------------------------------------------------------------------------


class PaymentOut(Base):
    id: str
    amount: Decimal
    payment_date: date
    method: str


class InvoiceSummary(Base):
    id: str
    status: str
    invoice_date: date
    due_date: date | None = None
    total_with_tax: Decimal
    amount_paid: Decimal
    amount_due: Decimal


class InvoiceDetail(InvoiceSummary):
    subtotal: Decimal
    cgst_amount: Decimal
    sgst_amount: Decimal
    igst_amount: Decimal
    notes: str | None = None
    payments: list[PaymentOut] = Field(default_factory=list)


class DisputeIn(Base):
    reason: str = Field(min_length=10, max_length=2000)


class DisputeOut(Base):
    id: str
    invoice_id: str
    status: str
    raised_at: datetime


# ---------------------------------------------------------------------------
# Notifications, profile, uploads
# ---------------------------------------------------------------------------


class NotificationOut(Base):
    id: str
    kind: str
    title: str
    body: str | None = None
    matter_id: str | None = None
    is_read: bool
    created_at: datetime


class ProfileOut(Base):
    id: str
    full_name: str
    # Read-only. A change of name or address goes through the firm, because the
    # firm's record is the one that matters on a filing.
    email: str
    phone: str | None = None
    client_name: str


class UploadOut(Base):
    id: str
    filename: str
    file_size_bytes: int
    scan_status: str
    status: str
    uploaded_at: datetime


class SignedUrlOut(Base):
    url: str
    expires_at: datetime


# ---------------------------------------------------------------------------
# Auth
# ---------------------------------------------------------------------------


class RequestOtpIn(Base):
    email: str = Field(max_length=320)


class RequestOtpOut(Base):
    # Always the same, known email or not — see spec §15.11.
    detail: str = "If that address is registered, a code has been sent."
    # Tests only; never populated when PORTAL_EXPOSE_OTP_FOR_TESTS is unset.
    debug_code: str | None = None


class VerifyOtpIn(Base):
    email: str = Field(max_length=320)
    code: str = Field(min_length=4, max_length=12)


class TokenOut(Base):
    access_token: str
    token_type: str = "bearer"
    expires_in: int
