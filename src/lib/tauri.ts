// Typed Tauri invoke() wrappers — the ONLY way Deck calls Keel.
// Never use raw invoke() in components. Always import from this file.
// One namespace per Keel command file.

import { invoke } from '@tauri-apps/api/core';
import type {
  AIRequestParams,
  AIResponse,
  Client,
  CreateClientInput,
  CreateDeadlineInput,
  CreateInvoiceInput,
  CreateMatterInput,
  CreateTimeEntryInput,
  Deadline,
  DeadlineSummary,
  CascadeAnchor,
  CascadePreview,
  DocumentMeta,
  Escalation,
  ExportedDocument,
  CreateIpAssetInput,
  FirmSettings,
  IpAsset,
  Invoice,
  InvoiceLineItem,
  InvoiceSummary,
  Matter,
  MatterFilter,
  MatterParty,
  MatterSummary,
  MatterType,
  Payment,
  RecordPaymentInput,
  InvitePortalUserInput,
  PortalUser,
  Session,
  StatutoryTemplate,
  SyncStatus,
  TimeEntry,
  UnbilledSummary,
  UpdateClientInput,
  UpdateDeadlineInput,
  UpdateFirmSettingsInput,
  UpcomingRenewal,
  UpdateIpAssetInput,
  UpdateMatterInput,
  UpdateTimeEntryInput,
  UploadDocumentInput,
  TemplateManifest,
  RenderDocumentInput,
  StagedAnnexureInfo,
  RenderResult,
} from './ipc-types';

// LoginInput not in spec yet — defined locally until auth-rbac.md spec lands
interface LoginInput { email: string; password: string; }

export const keel = {

  // ---- AI ---------------------------------------------------------------
  // THE ONLY AI ENTRY POINT. Use lib/ai.ts instead of calling this directly.
  ai: {
    request: (params: AIRequestParams) =>
      invoke<AIResponse>('ai_request', {
        taskType: params.taskType,
        context: params.context,
        prompt: params.prompt,
        deepAnalysis: params.deepAnalysis ?? false,
      }),
  },

  // ---- Auth -------------------------------------------------------------
  auth: {
    login: (input: LoginInput) => invoke<Session>('login', { input }),
    logout: () => invoke<void>('logout'),
    getSession: () => invoke<Session | null>('get_session'),
    /** Extend the session by another 8 hours. Null if it already lapsed. */
    refreshSession: () => invoke<Session | null>('refresh_session'),
  },

  // ---- Matters ----------------------------------------------------------
  matters: {
    get: (id: string) => invoke<Matter>('get_matter', { id }),
    list: (filter: MatterFilter) => invoke<MatterSummary[]>('list_matters', { filter }),
    create: (input: CreateMatterInput) => invoke<Matter>('create_matter', { input }),
    update: (id: string, input: UpdateMatterInput) => invoke<Matter>('update_matter', { id, input }),
    updateStatus: (id: string, status: string, note?: string) =>
      invoke<Matter>('update_matter_status', { id, status, note }),
    close: (id: string, reason: string) => invoke<Matter>('close_matter', { id, reason }),
    archive: (id: string) => invoke<Matter>('archive_matter', { id }),
    search: (query: string) => invoke<MatterSummary[]>('search_matters', { query }),
    assignParty: (matterId: string, userId: string, role: string) =>
      invoke<MatterParty>('assign_party', { matterId, userId, role }),
    removeParty: (matterId: string, userId: string) =>
      invoke<void>('remove_party', { matterId, userId }),
  },

  // ---- Clients ----------------------------------------------------------
  clients: {
    get: (id: string) => invoke<Client>('get_client', { id }),
    list: () => invoke<Client[]>('list_clients'),
    create: (input: CreateClientInput) => invoke<Client>('create_client', { input }),
    update: (id: string, input: UpdateClientInput) => invoke<Client>('update_client', { id, input }),
  },

  // ---- Deadlines --------------------------------------------------------
  deadlines: {
    listAll: () =>
      invoke<DeadlineSummary[]>('list_all_deadlines'),
    list: (matterId: string) =>
      invoke<Deadline[]>('list_deadlines', { matterId }),
    create: (input: CreateDeadlineInput) =>
      invoke<Deadline>('create_deadline', { input }),
    update: (id: string, input: UpdateDeadlineInput) =>
      invoke<Deadline>('update_deadline', { id, input }),
    markComplete: (id: string, notes: string) =>
      invoke<Deadline>('mark_deadline_complete', { id, notes }),
    delete: (id: string) =>
      invoke<void>('delete_deadline', { id }),
    getTemplates: (matterType: MatterType) =>
      invoke<StatutoryTemplate[]>('get_statutory_templates', { matterType }),
    /** Second-attorney sign-off. Keel rejects verifying your own deadline. */
    verify: (id: string) =>
      invoke<Deadline>('verify_deadline', { id }),
    /** Open statutory deadlines still awaiting a second pair of eyes. */
    listUnverified: () =>
      invoke<DeadlineSummary[]>('list_unverified_deadlines'),
  },

  // ---- IP Assets --------------------------------------------------------
  ipAssets: {
    list: (matterId: string) =>
      invoke<IpAsset[]>('list_ip_assets', { matterId }),
    get: (id: string) =>
      invoke<IpAsset>('get_ip_asset', { id }),
    create: (input: CreateIpAssetInput) =>
      invoke<IpAsset>('create_ip_asset', { input }),
    update: (id: string, input: UpdateIpAssetInput) =>
      invoke<IpAsset>('update_ip_asset', { id, input }),
    /** Rejected by Keel while deadlines still reference the asset. */
    delete: (id: string) =>
      invoke<void>('delete_ip_asset', { id }),
    /** Firm-wide renewals due within `withinDays` (default 365), plus lapsed. */
    upcomingRenewals: (withinDays?: number) =>
      invoke<UpcomingRenewal[]>('list_upcoming_renewals', { withinDays }),
  },

  // ---- Cascade + escalations --------------------------------------------
  cascade: {
    /** Show the chain an anchor would produce. Writes nothing. */
    preview: (anchor: CascadeAnchor) =>
      invoke<CascadePreview>('preview_cascade', { anchor }),
    /** Persist the chain. Refuses if already generated for that asset. */
    generate: (anchor: CascadeAnchor) =>
      invoke<number>('generate_cascade', { anchor }),
    /** Anchor event types that have a template, for the picker. */
    listAnchors: (ipType: string) =>
      invoke<string[]>('list_cascade_anchors', { ipType }),
  },

  escalations: {
    list: (includeResolved?: boolean) =>
      invoke<Escalation[]>('list_escalations', { includeResolved }),
    resolve: (id: string, action: string) =>
      invoke<void>('resolve_escalation', { id, action }),
  },

  // ---- Billing ----------------------------------------------------------
  billing: {
    getFirmSettings: () =>
      invoke<FirmSettings>('get_firm_settings'),
    updateFirmSettings: (input: UpdateFirmSettingsInput) =>
      invoke<FirmSettings>('update_firm_settings', { input }),
    createTimeEntry: (input: CreateTimeEntryInput) =>
      invoke<TimeEntry>('create_time_entry', { input }),
    updateTimeEntry: (id: string, input: UpdateTimeEntryInput) =>
      invoke<TimeEntry>('update_time_entry', { id, input }),
    deleteTimeEntry: (id: string) =>
      invoke<void>('delete_time_entry', { id }),
    listTimeEntries: (matterId?: string, userId?: string, dateFrom?: string, dateTo?: string) =>
      invoke<TimeEntry[]>('list_time_entries', { matterId, userId, dateFrom, dateTo }),
    getUnbilledSummary: (matterId: string) =>
      invoke<UnbilledSummary>('get_unbilled_summary', { matterId }),
    createInvoice: (input: CreateInvoiceInput) =>
      invoke<Invoice>('create_invoice', { input }),
    getInvoice: (id: string) =>
      invoke<Invoice>('get_invoice', { id }),
    listInvoices: (clientId?: string, status?: string, dateFrom?: string) =>
      invoke<InvoiceSummary[]>('list_invoices', { clientId, status, dateFrom }),
    listLineItems: (invoiceId: string) =>
      invoke<InvoiceLineItem[]>('list_invoice_line_items', { invoiceId }),
    listPayments: (invoiceId: string) =>
      invoke<Payment[]>('list_payments', { invoiceId }),
    updateInvoiceStatus: (id: string, status: string) =>
      invoke<Invoice>('update_invoice_status', { id, status }),
    recordPayment: (input: RecordPaymentInput) =>
      invoke<Payment>('record_payment', { input }),
    generateInvoicePdf: (id: string) =>
      invoke<string>('generate_invoice_pdf', { id }),
  },

  // ---- Documents --------------------------------------------------------
  documents: {
    list: (matterId: string) => invoke<DocumentMeta[]>('list_documents', { matterId }),
    upload: (input: UploadDocumentInput) => invoke<DocumentMeta>('upload_document', { input }),
    /** RAW bytes — internal use only (viewing, attorney working copies).
     *  Never send these to a client; use `exportForClient` for that. */
    get: (id: string) => invoke<number[]>('get_document', { id }), // bytes as number[]
    /** Metadata-stripped bytes for anything leaving the firm.
     *  Rejects file types Keel cannot clean rather than returning raw bytes. */
    exportForClient: (id: string) => invoke<ExportedDocument>('export_document', { id }),
    delete: (id: string) => invoke<void>('delete_document', { id }),
  },

  // ---- Sync + client portal (M5) ----------------------------------------
  sync: {
    status: () => invoke<SyncStatus>('sync_status'),
    /** Pushes the outbox, then pulls anything the client sent. Errors only if
     *  sync is off or unconfigured; a failed leg comes back in `lastError`. */
    trigger: () => invoke<SyncStatus>('trigger_sync'),
    /** Refused by Keel unless a server URL is configured. */
    setEnabled: (enabled: boolean) => invoke<SyncStatus>('set_sync_enabled', { enabled }),
    setServer: (url: string) => invoke<SyncStatus>('set_sync_server', { url }),
    /** Stored in the OS keychain, never in SQLite. Empty string clears it. */
    setToken: (token: string) => invoke<SyncStatus>('set_sync_token', { token }),
  },

  // ---- Drafting (M9.8 Smart Form Compiler) ------------------------------
  drafting: {
    /** Every template in the library, for the document-type picker. */
    listTemplates: () => invoke<TemplateManifest[]>('list_templates'),
    /** The schema Deck builds the form from. */
    getTemplate: (id: string) => invoke<TemplateManifest>('get_template', { id }),
    /** Validates, then renders. Draft is one pass for the preview; Final is two
     *  and, given a matterId, files the result into the vault. */
    render: (input: RenderDocumentInput) =>
      invoke<RenderResult>('render_document', { input }),
    /** Hand Keel a file the attorney picked. Keel reads, checks and cleans it,
     *  and holds it until the document is generated — so a preview that
     *  re-renders on every pause does not resend the bytes. */
    stageAnnexure: (path: string) =>
      invoke<StagedAnnexureInfo>('stage_annexure', { path }),
    discardAnnexure: (id: string) => invoke<void>('discard_annexure', { id }),
  },

  portalUsers: {
    list: (clientId: string) => invoke<PortalUser[]>('list_portal_users', { clientId }),
    invite: (input: InvitePortalUserInput) =>
      invoke<PortalUser>('invite_portal_user', { input }),
    /** Takes effect at the portal's next request, not at token expiry. */
    revoke: (id: string) => invoke<PortalUser>('revoke_portal_user', { id }),
  },

  sharing: {
    shareDocument: (documentId: string) =>
      invoke<void>('share_document', { documentId }),
    /** Queues a tombstone — removes the mirror row and the stored object. */
    unshareDocument: (documentId: string) =>
      invoke<void>('unshare_document', { documentId }),
    setDeadlineClientVisible: (id: string, visible: boolean) =>
      invoke<void>('set_deadline_client_visible', { id, visible }),
    /** What the client can currently see for a matter. */
    listSharedDocuments: (matterId: string) =>
      invoke<DocumentMeta[]>('list_shared_documents', { matterId }),
  },
};
