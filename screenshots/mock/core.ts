// Screenshot harness — stands in for @tauri-apps/api/core.
//
// The real app talks to Keel over Tauri IPC, which does not exist in a browser.
// This returns representative fixture data so the UI can be rendered and
// captured without a Tauri runtime.
//
// FIXTURES ONLY. Nothing here runs in the shipped app: it is wired in solely by
// vite.config.screenshots.ts and never by the normal build.

const today = new Date();
const iso = (offsetDays: number) => {
  const d = new Date(today);
  d.setDate(d.getDate() + offsetDays);
  return d.toISOString().slice(0, 10);
};
const stamp = (offsetDays: number) => `${iso(offsetDays)} 09:30:00`;

// ---------------------------------------------------------------------------
// Fixture data — a plausible day at a two-attorney Delhi IP firm
// ---------------------------------------------------------------------------

const CLIENTS = [
  { id: 'c-petal', name: 'Petalveda Scents Pvt Ltd', clientType: 'Company',
    email: 'legal@petalveda.in', phone: '+91 98100 11223', address: 'Okhla Phase II, New Delhi',
    gstin: '07AABCP1234M1Z5', pan: 'AABCP1234M', isActive: true,
    createdAt: stamp(-400), updatedAt: stamp(-30) },
  { id: 'c-arka', name: 'Arka Robotics LLP', clientType: 'Partnership',
    email: 'ip@arkarobotics.com', phone: '+91 99000 44556', address: 'Whitefield, Bengaluru',
    gstin: '29AAFCA9988K1Z2', pan: 'AAFCA9988K', isActive: true,
    createdAt: stamp(-300), updatedAt: stamp(-12) },
  { id: 'c-sundar', name: 'Sundaram Textiles', clientType: 'Company',
    email: 'admin@sundaramtex.in', phone: '+91 94440 77889', address: 'Tiruppur, Tamil Nadu',
    gstin: '33AACCS4455L1Z9', pan: 'AACCS4455L', isActive: true,
    createdAt: stamp(-220), updatedAt: stamp(-5) },
];

const MATTER_SUMMARIES = [
  { id: 'P&P-2026-TM-0042', title: 'PETALVEDA — word mark, classes 3 & 44',
    clientName: 'Petalveda Scents Pvt Ltd', matterType: 'Trademark', status: 'Active',
    priority: 'Urgent', responsibleAttorney: 'Sree Lakshmi Menon',
    nextDeadlineDate: iso(3), nextDeadlineEvent: 'Response to Examination Report',
    updatedAt: stamp(-1) },
  { id: 'P&P-2026-PT-0019', title: 'Autonomous pick-and-place manipulator',
    clientName: 'Arka Robotics LLP', matterType: 'Patent', status: 'Active',
    priority: 'High', responsibleAttorney: 'Kajal Thakur',
    nextDeadlineDate: iso(11), nextDeadlineEvent: 'Request for Examination',
    updatedAt: stamp(-2) },
  { id: 'P&P-2026-TM-0051', title: 'SUNVEIL — device mark, class 24',
    clientName: 'Sundaram Textiles', matterType: 'Trademark', status: 'PendingClientResponse',
    priority: 'Normal', responsibleAttorney: 'Kajal Thakur',
    nextDeadlineDate: iso(26), nextDeadlineEvent: 'Opposition period expires',
    updatedAt: stamp(-4) },
  { id: 'P&P-2026-DS-0007', title: 'Ergonomic bottle closure — design',
    clientName: 'Petalveda Scents Pvt Ltd', matterType: 'Design', status: 'Active',
    priority: 'Normal', responsibleAttorney: 'Sree Lakshmi Menon',
    nextDeadlineDate: iso(64), nextDeadlineEvent: 'Design renewal due (Form 6)',
    updatedAt: stamp(-9) },
  { id: 'P&P-2025-TM-0033', title: 'AASHNI — word mark, class 25',
    clientName: 'Sundaram Textiles', matterType: 'Trademark', status: 'Closed',
    priority: 'Normal', responsibleAttorney: 'Sree Lakshmi Menon',
    nextDeadlineDate: null, nextDeadlineEvent: null, updatedAt: stamp(-58) },
];

const MATTER = {
  id: 'P&P-2026-TM-0042',
  clientId: 'c-petal',
  title: 'PETALVEDA — word mark, classes 3 & 44',
  matterType: 'Trademark',
  subType: 'Prosecution',
  status: 'Active',
  priority: 'Urgent',
  responsiblePartnerId: 'user-slm',
  forum: 'Trade Marks Registry, Delhi',
  jurisdiction: 'India',
  openedDate: iso(-210),
  targetCloseDate: iso(400),
  internalNotes: 'Examiner has cited two prior marks; both are in class 3 only. '
    + 'Arguable that class 44 services are dissimilar. Client is fee-sensitive.',
  clientNotes: 'We have responded to the examination report and expect the Registry '
    + 'to advertise the mark in the Journal within 3–4 months.',
  tags: ['prosecution', 'fee-sensitive'],
  linkedMatterIds: ['P&P-2026-DS-0007'],
  parties: [
    { userId: 'user-slm', role: 'Partner',   isPrimary: true,  name: 'Sree Lakshmi Menon' },
    { userId: 'user-kt',  role: 'Associate', isPrimary: false, name: 'Kajal Thakur' },
  ],
  createdAt: stamp(-210), updatedAt: stamp(-1),
};

const DEADLINES = [
  { id: 'd-1', matterId: 'P&P-2026-TM-0042', ipAssetId: 'ip-petal',
    referenceNumber: 'P&P-DD-0087', docketingEvent: 'Response to Examination Report',
    eventType: 'Statutory', dueDate: iso(3), status: 'Pending', urgency: 'Critical',
    notes: 'Rule 45 — 30 days from date of notice.', completedAt: null, completedBy: null, isClientVisible: true,
    createdBy: 'user-kt', isVerified: false, verifiedBy: null, verifiedAt: null,
    createdAt: stamp(-27), updatedAt: stamp(-27) },
  { id: 'd-2', matterId: 'P&P-2026-TM-0042', ipAssetId: 'ip-petal',
    referenceNumber: 'P&P-DD-0088', docketingEvent: 'Internal: prepare Response to Examination Report',
    eventType: 'Procedural', dueDate: iso(-4), status: 'Complete', urgency: 'Normal',
    notes: '7 days before the statutory date.', completedAt: stamp(-5), completedBy: 'user-kt', isClientVisible: false,
    createdBy: 'user-kt', isVerified: false, verifiedBy: null, verifiedAt: null,
    createdAt: stamp(-27), updatedAt: stamp(-5) },
  { id: 'd-3', matterId: 'P&P-2026-TM-0042', ipAssetId: 'ip-petal',
    referenceNumber: 'P&P-DD-0091', docketingEvent: 'Opposition period expires',
    eventType: 'Statutory', dueDate: iso(112), status: 'Pending', urgency: 'Normal',
    notes: 's.21 — 4 months from advertisement.', completedAt: null, completedBy: null, isClientVisible: true,
    createdBy: 'user-kt', isVerified: true, verifiedBy: 'user-slm', verifiedAt: stamp(-20),
    createdAt: stamp(-25), updatedAt: stamp(-20) },
  { id: 'd-4', matterId: 'P&P-2026-TM-0042', ipAssetId: 'ip-petal',
    referenceNumber: 'P&P-DD-0092', docketingEvent: 'Trademark renewal due (10-year term)',
    eventType: 'Statutory', dueDate: iso(3400), status: 'Pending', urgency: 'Normal',
    notes: null, completedAt: null, completedBy: null, isClientVisible: true,
    createdBy: 'user-slm', isVerified: true, verifiedBy: 'user-kt', verifiedAt: stamp(-24),
    createdAt: stamp(-25), updatedAt: stamp(-24) },
];

const DEADLINE_SUMMARIES = [
  { id: 'd-x1', matterId: 'P&P-2026-TM-0042', matterTitle: 'PETALVEDA — word mark, classes 3 & 44',
    matterType: 'Trademark', clientName: 'Petalveda Scents Pvt Ltd',
    docketingEvent: 'Response to Examination Report', eventType: 'Statutory',
    dueDate: iso(-2), status: 'Pending', urgency: 'Overdue',
    notes: 'Registry notice dated last month.', updatedAt: stamp(-1) },
  { id: 'd-x2', matterId: 'P&P-2026-TM-0042', matterTitle: 'PETALVEDA — word mark, classes 3 & 44',
    matterType: 'Trademark', clientName: 'Petalveda Scents Pvt Ltd',
    docketingEvent: 'File counter-statement to opposition', eventType: 'Statutory',
    dueDate: iso(3), status: 'Pending', urgency: 'Critical', notes: null, updatedAt: stamp(-1) },
  { id: 'd-x3', matterId: 'P&P-2026-PT-0019', matterTitle: 'Autonomous pick-and-place manipulator',
    matterType: 'Patent', clientName: 'Arka Robotics LLP',
    docketingEvent: 'Request for Examination (RFE) due', eventType: 'Statutory',
    dueDate: iso(6), status: 'Pending', urgency: 'Warning',
    notes: 'Rule 24B — 48 months from priority.', updatedAt: stamp(-2) },
  { id: 'd-x4', matterId: 'P&P-2026-PT-0019', matterTitle: 'Autonomous pick-and-place manipulator',
    matterType: 'Patent', clientName: 'Arka Robotics LLP',
    docketingEvent: 'Internal: prepare RFE bundle', eventType: 'Procedural',
    dueDate: iso(11), status: 'Pending', urgency: 'Normal', notes: null, updatedAt: stamp(-2) },
  { id: 'd-x5', matterId: 'P&P-2026-TM-0051', matterTitle: 'SUNVEIL — device mark, class 24',
    matterType: 'Trademark', clientName: 'Sundaram Textiles',
    docketingEvent: 'Opposition period expires', eventType: 'Statutory',
    dueDate: iso(26), status: 'Pending', urgency: 'Normal', notes: null, updatedAt: stamp(-4) },
  { id: 'd-x6', matterId: 'P&P-2026-DS-0007', matterTitle: 'Ergonomic bottle closure — design',
    matterType: 'Design', clientName: 'Petalveda Scents Pvt Ltd',
    docketingEvent: 'Design renewal due (Form 6)', eventType: 'Statutory',
    dueDate: iso(64), status: 'Pending', urgency: 'Normal', notes: null, updatedAt: stamp(-9) },
];

const IP_ASSETS = [
  { id: 'ip-petal', matterId: 'P&P-2026-TM-0042', assetType: 'Trademark',
    title: 'PETALVEDA', applicationNumber: '5544121', registrationNumber: null,
    filingDate: iso(-210), priorityDate: null, grantDate: null, registrationDate: null,
    expiryDate: iso(3440), applicantEntityType: 'Startup', jurisdiction: 'India',
    classes: [3, 44], status: 'Examination', notes: null,
    createdAt: stamp(-210), updatedAt: stamp(-27) },
  { id: 'ip-petal-dev', matterId: 'P&P-2026-TM-0042', assetType: 'Trademark',
    title: 'PETALVEDA (device)', applicationNumber: '5544122', registrationNumber: null,
    filingDate: iso(-205), priorityDate: null, grantDate: null, registrationDate: null,
    expiryDate: iso(3445), applicantEntityType: 'Startup', jurisdiction: 'India',
    classes: [3], status: 'Advertised', notes: null,
    createdAt: stamp(-205), updatedAt: stamp(-30) },
];

const RENEWALS = [
  { id: 'ip-aashni', matterId: 'P&P-2025-TM-0033', assetType: 'Trademark', title: 'AASHNI',
    registrationNumber: '3312890', expiryDate: iso(-11), status: 'Registered',
    jurisdiction: 'India', matterTitle: 'AASHNI — word mark, class 25',
    clientName: 'Sundaram Textiles' },
  { id: 'ip-sunveil', matterId: 'P&P-2026-TM-0051', assetType: 'Trademark', title: 'SUNVEIL',
    registrationNumber: '4471002', expiryDate: iso(22), status: 'Registered',
    jurisdiction: 'India', matterTitle: 'SUNVEIL — device mark, class 24',
    clientName: 'Sundaram Textiles' },
  { id: 'ip-closure', matterId: 'P&P-2026-DS-0007', assetType: 'Design',
    title: 'Ergonomic bottle closure', registrationNumber: '355120', expiryDate: iso(64),
    status: 'Registered', jurisdiction: 'India',
    matterTitle: 'Ergonomic bottle closure — design', clientName: 'Petalveda Scents Pvt Ltd' },
  { id: 'ip-arka', matterId: 'P&P-2026-PT-0019', assetType: 'Patent',
    title: 'Autonomous pick-and-place manipulator', registrationNumber: null,
    expiryDate: iso(240), status: 'Granted', jurisdiction: 'India',
    matterTitle: 'Autonomous pick-and-place manipulator', clientName: 'Arka Robotics LLP' },
];

const ESCALATIONS = [
  { id: 'e-1', deadlineId: 'd-x1', escalationLevel: 4, triggeredAt: stamp(-1),
    resolutionAction: null, resolvedAt: null, resolvedBy: null,
    docketingEvent: 'Response to Examination Report', dueDate: iso(-2),
    matterId: 'P&P-2026-TM-0042', matterTitle: 'PETALVEDA — word mark, classes 3 & 44' },
  { id: 'e-2', deadlineId: 'd-x2', escalationLevel: 3, triggeredAt: stamp(0),
    resolutionAction: null, resolvedAt: null, resolvedBy: null,
    docketingEvent: 'File counter-statement to opposition', dueDate: iso(3),
    matterId: 'P&P-2026-TM-0042', matterTitle: 'PETALVEDA — word mark, classes 3 & 44' },
  { id: 'e-3', deadlineId: 'd-x3', escalationLevel: 2, triggeredAt: stamp(0),
    resolutionAction: null, resolvedAt: null, resolvedBy: null,
    docketingEvent: 'Request for Examination (RFE) due', dueDate: iso(6),
    matterId: 'P&P-2026-PT-0019', matterTitle: 'Autonomous pick-and-place manipulator' },
];

const DOCUMENTS = [
  { id: 'doc-1', matterId: 'P&P-2026-TM-0042', filename: 'Examination-Report-5544121.pdf',
    category: 'Correspondence', mimeType: 'application/pdf', fileSizeBytes: 284_113,
    version: 1, uploadedBy: 'user-kt', isSharedWithClient: true,
    description: 'Registry examination report', createdAt: stamp(-27), updatedAt: stamp(-27) },
  { id: 'doc-2', matterId: 'P&P-2026-TM-0042', filename: 'Draft-Response-v3.docx',
    category: 'Filing', mimeType: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    fileSizeBytes: 51_204, version: 3, uploadedBy: 'user-kt', isSharedWithClient: false,
    description: 'Working draft — tracked changes', createdAt: stamp(-6), updatedAt: stamp(-2) },
  { id: 'doc-3', matterId: 'P&P-2026-TM-0042', filename: 'POA-Petalveda-signed.pdf',
    category: 'Filing', mimeType: 'application/pdf', fileSizeBytes: 118_400,
    version: 1, uploadedBy: 'user-slm', isSharedWithClient: true,
    description: 'Power of attorney', createdAt: stamp(-200), updatedAt: stamp(-200) },
  { id: 'doc-4', matterId: 'P&P-2026-PT-0019', filename: 'Prior-art-search-report.pdf',
    category: 'SearchReport', mimeType: 'application/pdf', fileSizeBytes: 902_331,
    version: 2, uploadedBy: 'user-slm', isSharedWithClient: true,
    description: 'Freedom-to-operate search', createdAt: stamp(-90), updatedAt: stamp(-40) },
  { id: 'doc-5', matterId: 'P&P-2026-TM-0051', filename: 'Registration-Certificate-4471002.pdf',
    category: 'Certificate', mimeType: 'application/pdf', fileSizeBytes: 205_887,
    version: 1, uploadedBy: 'user-kt', isSharedWithClient: true,
    description: null, createdAt: stamp(-120), updatedAt: stamp(-120) },
];

const INVOICE_SUMMARIES = [
  { id: 'INV-2026-0014', clientId: 'c-petal', clientName: 'Petalveda Scents Pvt Ltd',
    status: 'Sent', invoiceDate: iso(-20), dueDate: iso(10),
    totalWithTax: 82_600, amountPaid: 0 },
  { id: 'INV-2026-0013', clientId: 'c-arka', clientName: 'Arka Robotics LLP',
    status: 'Paid', invoiceDate: iso(-48), dueDate: iso(-18),
    totalWithTax: 153_400, amountPaid: 153_400 },
  { id: 'INV-2026-0012', clientId: 'c-sundar', clientName: 'Sundaram Textiles',
    status: 'PartiallyPaid', invoiceDate: iso(-62), dueDate: iso(-32),
    totalWithTax: 47_200, amountPaid: 20_000 },
  { id: 'INV-2026-0011', clientId: 'c-petal', clientName: 'Petalveda Scents Pvt Ltd',
    status: 'Draft', invoiceDate: iso(-3), dueDate: iso(27),
    totalWithTax: 29_500, amountPaid: 0 },
];

const TIME_ENTRIES = [
  { id: 't-1', matterId: 'P&P-2026-TM-0042', userId: 'user-kt', date: iso(-1), hours: 2.5,
    description: 'Draft response to examination report; review cited marks',
    activityCode: 'L300', ratePerHour: 4000, isBillable: true, isInvoiced: false,
    invoiceId: null, createdAt: stamp(-1), updatedAt: stamp(-1) },
  { id: 't-2', matterId: 'P&P-2026-TM-0042', userId: 'user-slm', date: iso(-2), hours: 1.0,
    description: 'Call with client re: class 44 argument',
    activityCode: 'L700', ratePerHour: 8000, isBillable: true, isInvoiced: false,
    invoiceId: null, createdAt: stamp(-2), updatedAt: stamp(-2) },
  { id: 't-3', matterId: 'P&P-2026-PT-0019', userId: 'user-slm', date: iso(-3), hours: 3.25,
    description: 'Review prior art search; annotate claim chart',
    activityCode: 'L200', ratePerHour: 8000, isBillable: true, isInvoiced: true,
    invoiceId: 'INV-2026-0013', createdAt: stamp(-3), updatedAt: stamp(-3) },
];

const FIRM_SETTINGS = {
  firmName: 'Persistas & Partners', firmGstin: '07AAFCP7788K1Z3',
  firmAddress: 'C-42, Defence Colony\nNew Delhi 110024', firmPan: 'AAFCP7788K',
  bankName: 'HDFC Bank, Defence Colony', bankAccount: '50200012345678',
  bankIfsc: 'HDFC0000123', defaultHourlyRate: 5000, partnerRate: 8000,
  associateRate: 4000, paralegalRate: 2000, gstRate: 0.18, updatedAt: stamp(-30),
};

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

const HANDLERS: Record<string, (args: any) => unknown> = {
  get_session: () => ({
    sessionId: 'sess-demo', userId: 'user-slm', name: 'Sree Lakshmi Menon',
    role: 'Partner', email: 'slm@persist.in', expiresAt: stamp(0),
  }),
  refresh_session: () => null,
  logout: () => undefined,

  list_matters:  () => MATTER_SUMMARIES,
  search_matters: () => MATTER_SUMMARIES,
  get_matter:    () => MATTER,
  list_clients:  () => CLIENTS,
  get_client:    () => CLIENTS[0],

  list_all_deadlines: () => DEADLINE_SUMMARIES,
  list_deadlines:     () => DEADLINES,
  list_unverified_deadlines: () => DEADLINE_SUMMARIES.filter(d => d.eventType === 'Statutory'),
  get_statutory_templates: () => [
    { event: 'Response to Examination Report', eventType: 'Statutory',
      description: 'Rule 45 — 30 days from the date of the notice.', typicalDaysFromFiling: null },
    { event: 'Opposition period expires', eventType: 'Statutory',
      description: 's.21 — 4 months from advertisement in the Journal.', typicalDaysFromFiling: null },
  ],

  list_ip_assets: () => IP_ASSETS,
  list_upcoming_renewals: () => RENEWALS,
  list_cascade_anchors: () => ['TMApplication', 'TMExaminationReport', 'TMAdvertised'],
  preview_cascade: () => ({
    templateId: 'tpl-tm-application-in', anchorEvent: 'TMApplication',
    anchorDate: iso(-210), lastVerified: '2026-04-01',
    templateNotes: 'Trade Marks Act 1999. Renewal 10 years from filing; 6-month grace with surcharge.',
    deadlines: [
      { docketingEvent: 'Expect examination report', eventType: 'Procedural',
        dueDate: iso(155), isClientVisible: false, isInternalBuffer: false, notes: null },
      { docketingEvent: 'Internal: prepare Trademark renewal due (10-year term)',
        eventType: 'Procedural', dueDate: iso(3350), isClientVisible: false,
        isInternalBuffer: true, notes: '90 days before the statutory date.' },
      { docketingEvent: 'Trademark renewal due (10-year term)', eventType: 'Statutory',
        dueDate: iso(3440), isClientVisible: true, isInternalBuffer: false, notes: null },
      { docketingEvent: 'Internal: prepare Renewal grace period expires (with surcharge)',
        eventType: 'Procedural', dueDate: iso(3592), isClientVisible: false,
        isInternalBuffer: true, notes: '30 days before the statutory date.' },
      { docketingEvent: 'Renewal grace period expires (with surcharge)', eventType: 'Statutory',
        dueDate: iso(3622), isClientVisible: true, isInternalBuffer: false, notes: null },
    ],
  }),

  list_escalations: () => ESCALATIONS,

  // Scoped by matter, as Keel does — otherwise the all-firm vault view shows
  // the same fixtures repeated once per matter.
  list_documents: (args: any) =>
    DOCUMENTS.filter(d => d.matterId === args?.matterId),

  list_invoices:      () => INVOICE_SUMMARIES,
  list_time_entries:  () => TIME_ENTRIES,
  get_firm_settings:  () => FIRM_SETTINGS,
  get_unbilled_summary: () => ({ matterId: MATTER.id, matterTitle: MATTER.title,
    totalHours: 3.5, totalAmount: 18_000, entryCount: 2 }),

  // Two states worth seeing. `?sync=on` gives the configured firm — sync
  // running, a token stored, and a rejection sitting in last_error, which is
  // the state the Sync tab has to communicate well.
  sync_status: () =>
    new URLSearchParams(window.location.search).get('sync') === 'on'
      ? {
          lastSyncedAt: '2026-08-09 14:12:07', isSyncing: false, pendingChanges: 1,
          isEnabled: true, serverUrl: 'https://sync.persistas.in',
          lastError: '1 change(s) rejected: upsert deadline: matter P&P-2026-PT-0117 is not in the mirror',
          hasToken: true,
        }
      : {
          lastSyncedAt: null, isSyncing: false, pendingChanges: 3,
          isEnabled: false, serverUrl: null, lastError: null, hasToken: false,
        },

  list_portal_users: (args: any) => [
    { id: 'pu-1', clientId: args?.clientId ?? 'c-petal', fullName: 'Anita Rao',
      email: 'anita@petalveda.in', phone: '+91 98100 11223', status: 'Active',
      invitedBy: 'user-slm', invitedAt: stamp(-120), lastLoginAt: stamp(-2) },
    { id: 'pu-2', clientId: args?.clientId ?? 'c-petal', fullName: 'Ravi Menon',
      email: 'ravi@petalveda.in', phone: null, status: 'Invited',
      invitedBy: 'user-slm', invitedAt: stamp(-3), lastLoginAt: null },
    { id: 'pu-3', clientId: args?.clientId ?? 'c-petal', fullName: 'Former Secretary',
      email: 'old@petalveda.in', phone: null, status: 'Revoked',
      invitedBy: 'user-kt', invitedAt: stamp(-300), lastLoginAt: stamp(-95) },
  ],
};

export async function invoke<T>(cmd: string, args?: unknown): Promise<T> {
  const handler = HANDLERS[cmd];
  if (!handler) {
    // Loud rather than silent: an unmocked command should be obvious in the
    // console when a screenshot looks wrong.
    console.warn(`[screenshot-mock] no fixture for "${cmd}"`);
    return undefined as T;
  }
  return handler(args) as T;
}

export const convertFileSrc = (p: string) => p;
