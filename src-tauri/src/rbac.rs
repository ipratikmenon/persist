// Role-based access control — specs/auth-rbac.md §RBAC Rules.
//
// "RBAC enforcement: permission checks run in Keel at the command level...
//  Deck gates UI elements based on role stored in Zustand after login, but Keel
//  is the authoritative enforcer. Deck gating is UX convenience only — never
//  relied on for security."
//
// Until now the matrix was specified but nothing checked it: any signed-in user
// could close a matter or issue an invoice. This module is the enforcement.
//
// The design is deliberately a lookup table rather than scattered `if role ==`
// checks. The permission matrix lives in one place, reads like the spec, and can
// be tested exhaustively.

use crate::{AppState, SessionData};

// ---------------------------------------------------------------------------
// Roles
// ---------------------------------------------------------------------------

/// Seniority rank. Partner and Admin are equal — the spec's matrix gives them
/// identical rights, and inventing a difference here would be fiction.
fn rank(role: &str) -> u8 {
    match role {
        "Paralegal" => 1,
        "Associate" => 2,
        "Partner" | "Admin" => 3,
        // An unknown role gets nothing. Failing closed matters more than being
        // helpful about a role that should not exist.
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Permissions
// ---------------------------------------------------------------------------

/// Every gated action. Named after what the attorney is doing, not the command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    CreateMatter,
    EditMatter,
    CloseMatter,
    ArchiveMatter,

    UploadDocument,
    ShareDocumentWithClient,
    DeleteDocument,

    CreateDeadline,
    /// The second half of dual verification (spec §2.12).
    VerifyDeadline,
    WaiveDeadline,

    ViewAllBilling,
    CreateInvoice,
    EditFirmSettings,

    ManageUsers,
    ManagePortalUsers,
    ViewAnalytics,
}

impl Permission {
    /// Minimum rank required. Mirrors the table in specs/auth-rbac.md.
    fn required_rank(self) -> u8 {
        use Permission::*;
        match self {
            // Paralegal and up
            UploadDocument | CreateDeadline => 1,

            // Associate and up
            CreateMatter
            | EditMatter
            | ShareDocumentWithClient
            | VerifyDeadline => 2,

            // Partner / Admin only
            CloseMatter
            | ArchiveMatter
            | DeleteDocument
            | WaiveDeadline
            | ViewAllBilling
            | CreateInvoice
            | EditFirmSettings
            | ManageUsers
            | ManagePortalUsers
            | ViewAnalytics => 3,
        }
    }

    fn required_role_name(self) -> &'static str {
        match self.required_rank() {
            1 => "Paralegal",
            2 => "Associate",
            _ => "Partner",
        }
    }

    /// Whether a role satisfies this permission.
    pub fn allows(self, role: &str) -> bool {
        rank(role) >= self.required_rank()
    }
}

// ---------------------------------------------------------------------------
// Command-level guard
// ---------------------------------------------------------------------------

/// Assert the current session may perform `permission`, and return it.
///
/// Returns the session so callers can attribute the action (created_by,
/// verified_by) without a second lookup — the two things almost always go
/// together, and fetching them separately invites using one without the other.
pub async fn require(
    state: &tauri::State<'_, AppState>,
    permission: Permission,
) -> Result<SessionData, String> {
    let session = {
        let guard = state.session.lock().await;
        guard.clone()
    }
    .ok_or_else(|| "Not signed in".to_string())?;

    if !permission.allows(&session.role) {
        return Err(format!(
            "Insufficient permissions: {} required (you are {})",
            permission.required_role_name(),
            session.role
        ));
    }

    Ok(session)
}

/// The current session, without a permission check. For commands that only need
/// attribution — never use this in place of `require`.
pub async fn current_session(
    state: &tauri::State<'_, AppState>,
) -> Result<SessionData, String> {
    let guard = state.session.lock().await;
    guard.clone().ok_or_else(|| "Not signed in".to_string())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::Permission::*;
    use super::*;

    #[test]
    fn ranks_are_ordered() {
        assert!(rank("Paralegal") < rank("Associate"));
        assert!(rank("Associate") < rank("Partner"));
        assert_eq!(rank("Partner"), rank("Admin"), "spec gives them identical rights");
    }

    #[test]
    fn unknown_role_gets_nothing() {
        // Fail closed: a role that should not exist has no rights at all.
        for p in [UploadDocument, CreateDeadline, CreateMatter, CloseMatter] {
            assert!(!p.allows("Intern"), "{p:?} should be denied to an unknown role");
            assert!(!p.allows(""), "{p:?} should be denied to an empty role");
        }
    }

    #[test]
    fn paralegal_can_only_do_paralegal_things() {
        assert!(UploadDocument.allows("Paralegal"));
        assert!(CreateDeadline.allows("Paralegal"));

        assert!(!CreateMatter.allows("Paralegal"));
        assert!(!ShareDocumentWithClient.allows("Paralegal"));
        assert!(!VerifyDeadline.allows("Paralegal"));
        assert!(!CloseMatter.allows("Paralegal"));
        assert!(!CreateInvoice.allows("Paralegal"));
    }

    #[test]
    fn associate_can_work_but_not_close_or_bill() {
        assert!(CreateMatter.allows("Associate"));
        assert!(EditMatter.allows("Associate"));
        assert!(ShareDocumentWithClient.allows("Associate"));
        assert!(VerifyDeadline.allows("Associate"));

        assert!(!CloseMatter.allows("Associate"));
        assert!(!ArchiveMatter.allows("Associate"));
        assert!(!WaiveDeadline.allows("Associate"));
        assert!(!CreateInvoice.allows("Associate"));
        assert!(!EditFirmSettings.allows("Associate"));
        assert!(!ManageUsers.allows("Associate"));
        assert!(!DeleteDocument.allows("Associate"));
    }

    #[test]
    fn partner_and_admin_can_do_everything() {
        let all = [
            CreateMatter, EditMatter, CloseMatter, ArchiveMatter,
            UploadDocument, ShareDocumentWithClient, DeleteDocument,
            CreateDeadline, VerifyDeadline, WaiveDeadline,
            ViewAllBilling, CreateInvoice, EditFirmSettings,
            ManageUsers, ManagePortalUsers, ViewAnalytics,
        ];
        for p in all {
            assert!(p.allows("Partner"), "Partner should be allowed {p:?}");
            assert!(p.allows("Admin"),   "Admin should be allowed {p:?}");
        }
    }

    #[test]
    fn error_message_names_the_required_role() {
        assert_eq!(CloseMatter.required_role_name(), "Partner");
        assert_eq!(CreateMatter.required_role_name(), "Associate");
        assert_eq!(UploadDocument.required_role_name(), "Paralegal");
    }
}
