# Session S27 — 2026-08-11
## The schedule of payments, and a figure nobody types

---

## Summary

The notice's payment particulars were the last `computed` block with nothing
behind it. They are now a repeating list with a table around it and a total
under it — and the total is summed from the rows rather than typed beside them.

**Result:** cargo test **324/324** with `PERSIST_REQUIRE_LATEX=1` (was 311),
Deck build PASS, 23 screenshots. On `claude/new-session-dbqe5o`.

I could not check the tranche fields against your own notice: the PDF you
uploaded went with the container between sessions. The shape below is from what
the repo had recorded about it (seven tranches, WhatsApp payment screenshots as
proof) plus ordinary Indian practice, and it is the part of this you should look
at hardest.

---

## 1. What a tranche holds

| Field | Kind | Why |
|---|---|---|
| Date paid | date | Set as "11 February 2026", not as an ISO string |
| Amount | number, min 1 | The column the total is summed from |
| Paid by | select — UPI / Bank transfer / Cheque / Cash / Card | A closed list; the mode is disputed often enough to be worth stating exactly |
| Reference | text, optional | UTR, cheque number, or the annexure the proof is at |

Set as a table, not as prose. Seven tranches in a paragraph is a paragraph
nobody reads, and a recipient disputing one of them needs to be able to point at
the row.

---

## 2. The total is derived

A demand notice that states a sum and then schedules the payments making it up
has said the same thing twice. The day the two disagree is the day the notice is
worth arguing about, so the attorney enters the tranches and the figure follows.

Two things were needed on a list to do that without another Rust special case:

**`blockTemplate`** — how the rows are wrapped, once, when there is at least
one. A table needs its header row once, not once per row. And a list with no
rows now assembles to nothing at all, rather than to column headings over blank
paper with "Total paid: ₹0.00" under them.

**`totalOf`** — which item field `{{TOTAL}}` sums.

I built this first as a separate `FieldKind::Sum` and replaced it. A sum as its
own field cannot vanish together with the table it belongs to: the schedule
would disappear on an empty list and the total would still print zero. Folding
both into the list made the empty case correct by construction rather than by a
second condition.

---

## 3. A cell is set according to what it is

The first compile put `7000` in a row and `1,04,000.00` in the total under it —
arithmetically right, and a document that looks like it is quoting two different
figures. The dates read `2026-02-11` in a notice whose dateline says "14th July
2026".

So the manifest's declared kind decides the formatting: the totalled column is
money by definition, and a `date` cell holds an ISO string no legal document
prints as it stands.

That needed two formatters that already existed in one place each:

| Extracted | From | Because |
|---|---|---|
| `services/money.rs` | `commands/billing.rs` | A GST invoice reading "1,55,760.00" beside a notice reading "155,760.00" is two documents from one firm that do not agree |
| `services/dates.rs` | `services/firm.rs` | A notice dated "14th July 2026" over a schedule dated "2026-02-11" is one document in two registers |

`firm.rs` keeps the superscripted form for the dateline, where the eye rests on
it; the table gets the plain long date, because a superscript in a narrow column
is fussy.

---

## 4. Drift checking

Four new ways a manifest can be wrong, each silent otherwise:

- a `blockTemplate` with no `{{ROWS}}` — every row dropped from the document
- a `{{TOTAL}}` with no `totalOf` to sum
- a `totalOf` naming a column the row does not have
- a `totalOf` naming a column that is not a number — the total is zero, and
  nothing says so

---

## 5. Negative controls

| Control | Result |
|---|---|
| Dropped `{{ROWS}}` from the wrapper | 2 tests failed, including the compile: `2026-02-11 is not in the schedule` |
| Totalled without Indian grouping | 2 failed. The output is the point: the subject line still read `₹1,04,000` while the total would have printed `104000.00` — the two-figures problem, on the page |
| Pointed a computed template at an undeclared key (S26 control, re-run) | The shipped-library drift test named both halves |

---

## 6. The harness had a stale copy of a shipped manifest

`screenshots/mock/core.ts` had the real manifests **pasted into it**, so adding
a field to the notice left the harness drawing a form the app no longer has. The
capture failed looking for a button that exists in the app. They are imported
from `src-tauri/storage/templates/` now, and cannot go stale again.

Also: Playwright lives in a scratch install at `/tmp/shotkit`, outside the repo,
and went with the container. `capture.mjs` failed with a module-resolution
error; it now says how to put it back.

---

## 7. Files

```
src-tauri/src/services/money.rs        new — Indian digit grouping, out of billing
src-tauri/src/services/dates.rs        new — the long and ordinal date forms, out of firm.rs
src-tauri/src/services/templates.rs    blockTemplate, totalOf, kind-driven cell formatting, drift checks
src-tauri/src/services/firm.rs         uses services/dates
src-tauri/src/commands/billing.rs      uses services/money
storage/templates/legal-notice.{tex,json}          PAYMENTS_BLOCK
storage/templates/_shared/persist-letterhead.tex   paymentschedule, paymentrow, paymenttotal
src/lib/ipc-types.ts                   the list kind can carry a wrapper and a total
screenshots/{mock/core.ts,capture.mjs} manifests imported; Playwright path explained
```

---

## 8. Open

- **The tranche fields are my design, not your notice's.** Worth ten minutes of
  your time before this is used.
- The boilerplate notice language is still my reconstruction, and has not had
  your read-through.
- No autofill of a list row from the matter record.
- Staged annexures are held in memory, not separately vaulted.
