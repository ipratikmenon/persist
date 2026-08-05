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
  DocumentMeta,
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
  Session,
  StatutoryTemplate,
  SyncStatus,
  TimeEntry,
  UnbilledSummary,
  UpdateClientInput,
  UpdateDeadlineInput,
  UpdateFirmSettingsInput,
  UpdateIpAssetInput,
  UpdateMatterInput,
  UpdateTimeEntryInput,
  UploadDocumentInput,
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
    get: (id: string) => invoke<number[]>('get_document', { id }), // bytes as number[]
    delete: (id: string) => invoke<void>('delete_document', { id }),
  },

  // ---- Sync -------------------------------------------------------------
  sync: {
    status: () => invoke<SyncStatus>('sync_status'),
    trigger: () => invoke<SyncStatus>('trigger_sync'),
  },
};
