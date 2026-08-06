// IPC types — TypeScript interfaces that MUST stay in sync with Rust command return types.
// When you change a Keel command's return type, update this file immediately.
// Source of truth: src-tauri/src/commands/

// ---------------------------------------------------------------------------
// AI
// ---------------------------------------------------------------------------

export interface AIRequestParams {
  taskType: string;
  context: string;
  prompt: string;
  deepAnalysis?: boolean;
}

export interface AIResponse {
  content: string;
  /** model_used is for logging only — never display this to users */
  modelUsed: string;
  taskType: string;
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

export type UserRole = 'Partner' | 'Associate' | 'Paralegal' | 'Admin';

export interface Session {
  /** Session token — primary key of the `sessions` row in Keel. */
  sessionId: string;
  userId: string;
  name: string;
  role: UserRole;
  email: string;
  /** SQLite datetime string, UTC. Session is invalid at/after this instant. */
  expiresAt: string;
}

// ---------------------------------------------------------------------------
// Clients
// ---------------------------------------------------------------------------

export type ClientType = 'Individual' | 'Company' | 'Partnership' | 'Trust' | 'Other';

export interface Client {
  id: string;
  name: string;
  clientType: ClientType;  // Rust: client_type → camelCase → clientType
  email: string | null;
  phone: string | null;
  address: string | null;
  gstin: string | null;
  pan: string | null;
  isActive: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CreateClientInput {
  name: string;
  clientType: ClientType;  // Rust: client_type → camelCase → clientType
  email?: string;
  phone?: string;
  address?: string;
  gstin?: string;
  pan?: string;
}

export interface UpdateClientInput {
  name?: string;
  clientType?: ClientType;  // Rust: client_type → camelCase → clientType
  email?: string;
  phone?: string;
  address?: string;
  gstin?: string;
  pan?: string;
  isActive?: boolean;
}

// ---------------------------------------------------------------------------
// Matters
// ---------------------------------------------------------------------------

export type MatterType =
  | 'Trademark'
  | 'Patent'
  | 'Design'
  | 'Copyright'
  | 'Corporate'
  | 'Litigation'
  | 'Paralegal';

export type MatterStatus =
  | 'Active'
  | 'OnHold'
  | 'PendingClientResponse'
  | 'Closed'
  | 'Archived';

export type MatterPriority = 'Normal' | 'High' | 'Urgent';

export interface MatterParty {
  userId: string;
  role: UserRole;
  isPrimary: boolean;
  name: string; // denormalised for display
}

export interface Matter {
  id: string; // P&P-YYYY-TYPE-NNNN
  clientId: string;
  title: string;
  matterType: MatterType;
  subType: string | null;
  status: MatterStatus;
  priority: MatterPriority;
  responsiblePartnerId: string | null;
  forum: string | null;
  jurisdiction: string;
  openedDate: string;
  targetCloseDate: string | null;
  internalNotes: string | null; // NEVER shown in client portal
  clientNotes: string | null;
  tags: string[];
  linkedMatterIds: string[];
  parties: MatterParty[];
  createdAt: string;
  updatedAt: string;
}

export interface MatterSummary {
  id: string;
  title: string;
  clientName: string;
  matterType: MatterType;
  status: MatterStatus;
  priority: MatterPriority;
  responsibleAttorney: string | null;
  nextDeadlineDate: string | null;
  nextDeadlineEvent: string | null;
  updatedAt: string;
}

export interface MatterFilter {
  status?: MatterStatus[];
  matterType?: MatterType[];
  responsibleUserId?: string;
  clientId?: string;
  priority?: MatterPriority[];
}

export interface CreateMatterInput {
  clientId: string;
  title: string;
  matterType: MatterType;
  subType?: string;
  priority?: MatterPriority;
  forum?: string;
  jurisdiction?: string;
  openedDate: string;
  targetCloseDate?: string;
  internalNotes?: string;
  clientNotes?: string;
  tags?: string[];
}

export interface UpdateMatterInput {
  title?: string;
  subType?: string;
  priority?: MatterPriority;
  forum?: string;
  targetCloseDate?: string;
  internalNotes?: string;
  clientNotes?: string;
  tags?: string[];
  linkedMatterIds?: string[];
}

// ---------------------------------------------------------------------------
// Deadlines
// ---------------------------------------------------------------------------

/** 'Missed' is terminal and set only by abandonment_watcher, never by a user. */
export type DeadlineStatus = 'Pending' | 'Complete' | 'Waived' | 'Missed';
export type UrgencyTier = 'Overdue' | 'Critical' | 'Warning' | 'Normal';
export type EventType = 'Statutory' | 'Procedural' | 'Custom';

export interface Deadline {
  id: string;
  matterId: string;
  /** Set when the deadline belongs to a specific IP asset; null for matter-level ones. */
  ipAssetId: string | null;
  /** P&P-DD-NNNN — citable in correspondence. */
  referenceNumber: string | null;
  docketingEvent: string;
  eventType: EventType;
  dueDate: string;
  status: DeadlineStatus;
  urgency: UrgencyTier;
  notes: string | null;
  completedAt: string | null;
  completedBy: string | null;
  /** Who entered the date. Dual verification requires a different verifier. */
  createdBy: string | null;
  isVerified: boolean;
  verifiedBy: string | null;
  verifiedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

/** Denormalised view for DocketList — includes matter + client name. */
export interface DeadlineSummary {
  id: string;
  matterId: string;
  matterTitle: string;
  matterType: MatterType;
  clientName: string;
  docketingEvent: string;
  eventType: EventType;
  dueDate: string;
  status: DeadlineStatus;
  urgency: UrgencyTier;
  notes: string | null;
  updatedAt: string;
}

/** Standard statutory/procedural template for a given matter type. */
export interface StatutoryTemplate {
  event: string;
  eventType: Exclude<EventType, 'Custom'>;
  description: string;
  typicalDaysFromFiling: number | null;
}

export interface CreateDeadlineInput {
  matterId: string;
  ipAssetId?: string;
  // createdBy is set by Keel from the session — never sent by Deck.
  docketingEvent: string;
  eventType?: EventType;
  dueDate: string;
  notes?: string;
}

// ---------------------------------------------------------------------------
// IP Assets — Phase 1 Module 2 extended (B02)
// ---------------------------------------------------------------------------

export type IpAssetType =
  | 'Trademark'
  | 'Patent'
  | 'Design'
  | 'Copyright'
  | 'PlantVariety';

export type ApplicantEntityType =
  | 'Individual'
  | 'Startup'
  | 'SmallEntity'
  | 'Company'
  | 'Government';

export type IpAssetStatus =
  | 'Pending'
  | 'Examination'
  | 'Accepted'
  | 'Advertised'
  | 'Opposed'
  | 'Registered'
  | 'Granted'
  | 'Lapsed'
  | 'Abandoned'
  | 'Cancelled';

export interface IpAsset {
  id: string;
  matterId: string;
  assetType: IpAssetType;
  title: string;
  applicationNumber: string | null;
  registrationNumber: string | null;
  filingDate: string | null;
  priorityDate: string | null;
  grantDate: string | null;
  registrationDate: string | null;
  expiryDate: string | null;
  applicantEntityType: ApplicantEntityType;
  jurisdiction: string;
  /** Nice (trademark) or Locarno (design) class numbers. */
  classes: number[];
  status: IpAssetStatus;
  notes: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CreateIpAssetInput {
  matterId: string;
  assetType: IpAssetType;
  title: string;
  applicationNumber?: string;
  registrationNumber?: string;
  filingDate?: string;
  priorityDate?: string;
  grantDate?: string;
  registrationDate?: string;
  expiryDate?: string;
  applicantEntityType?: ApplicantEntityType;
  jurisdiction?: string;
  classes?: number[];
  status?: IpAssetStatus;
  notes?: string;
}

/** Every field optional — only what is supplied is written. */
// ---------------------------------------------------------------------------
// Cascade + escalations — Phase 1 Module 2 extended
// ---------------------------------------------------------------------------

/** The event a statutory chain is generated from. Always a real, dated event. */
export interface CascadeAnchor {
  matterId: string;
  ipAssetId: string;
  /** e.g. 'TMApplication', 'PatentFER' — must have a cascade_templates row. */
  eventType: string;
  /** YYYY-MM-DD — the date the anchor event actually occurred. */
  anchorDate: string;
}

export interface ProposedDeadline {
  docketingEvent: string;
  eventType: EventType;
  dueDate: string;
  isClientVisible: boolean;
  /** True for the firm's internal working date ahead of a statutory one. */
  isInternalBuffer: boolean;
  notes: string | null;
}

export interface CascadePreview {
  templateId: string;
  anchorEvent: string;
  anchorDate: string;
  /** When the statutory periods were last confirmed against the Act. */
  lastVerified: string;
  templateNotes: string | null;
  deadlines: ProposedDeadline[];
}

/** 1 = 14 days out, 2 = 7 days, 3 = 3 days, 4 = missed. */
export type EscalationLevel = 1 | 2 | 3 | 4;

export interface Escalation {
  id: string;
  deadlineId: string;
  escalationLevel: EscalationLevel;
  triggeredAt: string;
  resolutionAction: string | null;
  resolvedAt: string | null;
  resolvedBy: string | null;
  docketingEvent: string;
  dueDate: string;
  matterId: string;
  matterTitle: string;
}

/** Firm-wide renewal row for the Renewal Dashboard. */
export interface UpcomingRenewal {
  id: string;
  matterId: string;
  assetType: IpAssetType;
  title: string;
  registrationNumber: string | null;
  expiryDate: string;
  status: IpAssetStatus;
  jurisdiction: string;
  matterTitle: string;
  clientName: string;
}

export interface UpdateIpAssetInput {
  assetType?: IpAssetType;
  title?: string;
  applicationNumber?: string;
  registrationNumber?: string;
  filingDate?: string;
  priorityDate?: string;
  grantDate?: string;
  registrationDate?: string;
  expiryDate?: string;
  applicantEntityType?: ApplicantEntityType;
  jurisdiction?: string;
  classes?: number[];
  status?: IpAssetStatus;
  notes?: string;
}

export interface UpdateDeadlineInput {
  docketingEvent?: string;
  dueDate?: string;
  notes?: string;
}

// ---------------------------------------------------------------------------
// Documents
// ---------------------------------------------------------------------------

export type DocumentCategory =
  | 'Correspondence'
  | 'Filing'
  | 'Certificate'
  | 'SearchReport'
  | 'Invoice'
  | 'Contract'
  | 'Other';

export interface DocumentMeta {
  id: string;
  matterId: string;
  filename: string;
  category: DocumentCategory;
  mimeType: string;
  fileSizeBytes: number;
  version: number;
  uploadedBy: string;
  isSharedWithClient: boolean;
  description: string | null;
  createdAt: string;
  updatedAt: string;
  // vault_path intentionally absent — never sent from Keel to Deck
}

/** What a client-facing export stripped out of a document. */
export interface CleanReport {
  /** Human-readable list, e.g. "3 tracked change(s)", "GPS location data". */
  removed: string[];
  originalBytes: number;
  cleanedBytes: number;
}

/** Result of `export_document` — metadata-stripped bytes plus the report. */
export interface ExportedDocument {
  filename: string;
  /** Cleaned bytes, as a number array over the Tauri IPC bridge. */
  bytes: number[];
  report: CleanReport;
}

export interface UploadDocumentInput {
  matterId: string;
  filename: string;
  category: DocumentCategory;
  mimeType?: string;
  description?: string;
  /** Raw file bytes — Deck reads via Tauri FS plugin and passes here. */
  bytes: number[];
}

// ---------------------------------------------------------------------------
// Billing (Phase 2 M4)
// ---------------------------------------------------------------------------

export type ActivityCode = 'L100' | 'L200' | 'L300' | 'L400' | 'L500' | 'L600' | 'L700' | 'L800' | 'L900';
export type InvoiceStatus = 'Draft' | 'Sent' | 'Paid' | 'PartiallyPaid' | 'Cancelled';
export type PaymentMethod = 'BankTransfer' | 'Cheque' | 'Cash' | 'UPI' | 'NEFT' | 'RTGS';
export type GstType = 'Intra' | 'Inter';

export const ACTIVITY_CODES: { code: ActivityCode; label: string }[] = [
  { code: 'L100', label: 'Consultation' },
  { code: 'L200', label: 'Research' },
  { code: 'L300', label: 'Drafting' },
  { code: 'L400', label: 'Filing' },
  { code: 'L500', label: 'Review' },
  { code: 'L600', label: 'Hearing' },
  { code: 'L700', label: 'Client Comm.' },
  { code: 'L800', label: 'Administrative' },
  { code: 'L900', label: 'Travel' },
];

export interface FirmSettings {
  firmName: string;
  firmGstin: string | null;
  firmAddress: string | null;
  firmPan: string | null;
  bankName: string | null;
  bankAccount: string | null;
  bankIfsc: string | null;
  defaultHourlyRate: number;
  partnerRate: number;
  associateRate: number;
  paralegalRate: number;
  gstRate: number;
  updatedAt: string;
}

export interface TimeEntry {
  id: string;
  matterId: string;
  userId: string;
  date: string;
  hours: number;
  description: string;
  activityCode: ActivityCode;
  ratePerHour: number;
  amount: number;       // hours * ratePerHour — computed by Keel
  isBillable: boolean;
  isInvoiced: boolean;
  invoiceId: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface Invoice {
  id: string;
  clientId: string;
  matterIds: string[];
  status: InvoiceStatus;
  invoiceDate: string;
  dueDate: string | null;
  subtotal: number;
  cgstAmount: number;
  sgstAmount: number;
  igstAmount: number;
  totalWithTax: number;
  amountPaid: number;
  balanceDue: number;   // totalWithTax - amountPaid — computed by Keel
  notes: string | null;
  gstType: GstType;
  pdfDocId: string | null;
  createdBy: string;
  createdAt: string;
  updatedAt: string;
}

export interface InvoiceSummary {
  id: string;
  clientName: string;
  status: InvoiceStatus;
  invoiceDate: string;
  dueDate: string | null;
  totalWithTax: number;
  amountPaid: number;
  balanceDue: number;
  isOverdue: boolean;
}

export interface InvoiceLineItem {
  id: string;
  invoiceId: string;
  timeEntryId: string | null;
  description: string;
  activityCode: string | null;
  hours: number | null;
  rate: number;
  amount: number;
  sortOrder: number;
}

export interface Payment {
  id: string;
  invoiceId: string;
  amount: number;
  paymentDate: string;
  method: PaymentMethod;
  reference: string | null;
  notes: string | null;
  recordedBy: string;
  createdAt: string;
}

export interface UnbilledSummary {
  matterId: string;
  totalHours: number;
  totalAmount: number;
  entryCount: number;
}

export interface CreateTimeEntryInput {
  matterId: string;
  date: string;
  hours: number;
  description: string;
  activityCode?: ActivityCode;
  isBillable?: boolean;
}

export interface UpdateTimeEntryInput {
  date?: string;
  hours?: number;
  description?: string;
  activityCode?: ActivityCode;
  isBillable?: boolean;
}

export interface LineItemInput {
  timeEntryId?: string;
  description: string;
  activityCode?: string;
  hours?: number;
  rate: number;
  amount: number;
  sortOrder: number;
}

export interface CreateInvoiceInput {
  clientId: string;
  matterIds: string[];
  invoiceDate: string;
  dueDate?: string;
  gstType: GstType;
  lineItems: LineItemInput[];
  notes?: string;
}

export interface RecordPaymentInput {
  invoiceId: string;
  amount: number;
  paymentDate: string;
  method: PaymentMethod;
  reference?: string;
  notes?: string;
}

export interface UpdateFirmSettingsInput {
  firmName?: string;
  firmGstin?: string;
  firmAddress?: string;
  firmPan?: string;
  bankName?: string;
  bankAccount?: string;
  bankIfsc?: string;
  partnerRate?: number;
  associateRate?: number;
  paralegalRate?: number;
}

// ---------------------------------------------------------------------------
// Sync
// ---------------------------------------------------------------------------

export interface SyncStatus {
  lastSyncedAt: string | null;
  isSyncing: boolean;
  pendingChanges: number;
  /** False until a server URL is set and sync is deliberately enabled. */
  isEnabled: boolean;
  serverUrl: string | null;
  lastError: string | null;
}

// ---------------------------------------------------------------------------
// Client portal — Phase 2 Module 5
// ---------------------------------------------------------------------------

export type PortalUserStatus = 'Invited' | 'Active' | 'Suspended' | 'Revoked';

/** A person with portal access. One email maps to exactly one client. */
export interface PortalUser {
  id: string;
  clientId: string;
  fullName: string;
  email: string;
  phone: string | null;
  status: PortalUserStatus;
  invitedBy: string;
  invitedAt: string;
  lastLoginAt: string | null;
}

export interface InvitePortalUserInput {
  clientId: string;
  fullName: string;
  email: string;
  phone?: string;
}
