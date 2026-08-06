// UI permission gating — mirrors src-tauri/src/rbac.rs.
//
// KEEL IS THE AUTHORITATIVE ENFORCER. Per specs/auth-rbac.md:
// "Deck gates UI elements based on role stored in Zustand after login, but Keel
//  is the authoritative enforcer. Deck gating is UX convenience only — never
//  relied on for security."
//
// So this file exists to avoid showing an attorney a button that will refuse
// them — not to protect anything. Every gated action is checked again in Keel,
// and a mismatch between the two is a UX bug, not a security hole.
//
// Keep the ranks and the matrix in step with rbac.rs by hand. They are small,
// they change rarely, and a divergence degrades to "button shown, command
// refused" rather than to a leak.

import type { UserRole } from './ipc-types';

/** Seniority rank. Partner and Admin are equal, as in rbac.rs. */
const RANK: Record<string, number> = {
  Paralegal: 1,
  Associate: 2,
  Partner:   3,
  Admin:     3,
};

export type Permission =
  | 'CreateMatter'
  | 'EditMatter'
  | 'CloseMatter'
  | 'ArchiveMatter'
  | 'UploadDocument'
  | 'ShareDocumentWithClient'
  | 'DeleteDocument'
  | 'CreateDeadline'
  | 'VerifyDeadline'
  | 'WaiveDeadline'
  | 'ViewAllBilling'
  | 'CreateInvoice'
  | 'EditFirmSettings'
  | 'ManageUsers'
  | 'ManagePortalUsers'
  | 'ViewAnalytics';

const REQUIRED_RANK: Record<Permission, number> = {
  UploadDocument:          1,
  CreateDeadline:          1,

  CreateMatter:            2,
  EditMatter:              2,
  ShareDocumentWithClient: 2,
  VerifyDeadline:          2,

  CloseMatter:             3,
  ArchiveMatter:           3,
  DeleteDocument:          3,
  WaiveDeadline:           3,
  ViewAllBilling:          3,
  CreateInvoice:           3,
  EditFirmSettings:        3,
  ManageUsers:             3,
  ManagePortalUsers:       3,
  ViewAnalytics:           3,
};

/**
 * Whether a role may perform an action. An unknown role gets rank 0 and is
 * denied everything — the same fail-closed default as Keel.
 */
export function can(role: UserRole | string | undefined, permission: Permission): boolean {
  const rank = RANK[role ?? ''] ?? 0;
  return rank >= REQUIRED_RANK[permission];
}

/** The role a user would need, for an explanatory tooltip. */
export function requiredRole(permission: Permission): string {
  switch (REQUIRED_RANK[permission]) {
    case 1:  return 'Paralegal';
    case 2:  return 'Associate';
    default: return 'Partner';
  }
}
