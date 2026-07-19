// Global keyboard shortcuts for Persist Desktop.
// Module-specific shortcuts are registered on mount and unregistered on unmount.
// Global shortcuts use Tauri's global-shortcut plugin (registered in lib.rs setup).

export const SHORTCUTS = {
  PERSIST_CHAT:      'CmdOrCtrl+/',   // Open Hummingbird chat overlay
  COMMAND_PALETTE:   'CmdOrCtrl+K',   // Open command palette
  NEW_MATTER:        'CmdOrCtrl+N',   // New matter
  SEARCH:            'CmdOrCtrl+F',   // Global search

  // Mail module (registered on module mount)
  MAIL_DONE:         'E',             // Mark email done
  MAIL_SNOOZE:       'S',             // Snooze
  MAIL_PIN:          'P',             // Pin
  MAIL_REPLY:        'R',             // Reply
  MAIL_FORWARD:      'F',             // Forward

  // Editor shortcuts are handled by ProseMirror internally
} as const;

export type ShortcutKey = keyof typeof SHORTCUTS;
