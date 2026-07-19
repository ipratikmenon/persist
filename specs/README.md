# specs/

One spec file per module, written and approved before implementation begins.

## Status

| Module | Spec file | Status |
|---|---|---|
| Module 1 — Matter Management | `module-01-matters.md` | ✅ Approved |
| Module 2 — Docketing & Deadline Engine | `module-02-docketing.md` | ❌ Not written |
| Module 3 — Document Management | `module-03-documents.md` | ✅ Approved — File Manager Tree planned as M3.1 |
| Auth + RBAC | `auth-rbac.md` | ❌ Not written |
| Module 4 — Time Tracking & Billing | `module-04-billing.md` | ❌ Not written |
| Module 5 — Client Portal | `module-05-portal.md` | ❌ Not written |

## Rules

- Write the spec before writing any code for that module.
- Spec must be reviewed and approved before Step 2 (schema session) begins.
- Do not implement anything not in the spec — add it to the spec first.
- Never delete spec files — they are the contract the code was written against.
