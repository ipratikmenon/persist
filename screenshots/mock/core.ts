// Screenshot harness — stands in for @tauri-apps/api/core.
//
// The real app talks to Keel over Tauri IPC, which does not exist in a browser.
// This returns representative fixture data so the UI can be rendered and
// captured without a Tauri runtime.
//
// FIXTURES ONLY. Nothing here runs in the shipped app: it is wired in solely by
// vite.config.screenshots.ts and never by the normal build.

// Apply the defaults serde fills in on the way out of Keel. The manifests on
// disk omit `required` and `inputOnly` where they take the default value, so
// serving the raw JSON would show every field as optional — a mock that is
// wrong in the same way twice is worse than no mock.
function withDefaults(manifest: any) {
  return {
    ...manifest,
    fields: manifest.fields.map((f: any) => ({
      required: true,
      inputOnly: false,
      help: null,
      autofill: null,
      shownWhen: null,
      ...f,
    })),
  };
}

const TEMPLATES: any[] = [];

// Real shipped manifests, so the generated form is the real form.
const TM_REPLY_MANIFEST: any = {
  "id": "tm-examination-reply",
  "name": "Reply to Examination Report",
  "category": "IP / Trademark",
  "version": 1,
  "revised": "2026-08-10",
  "authority": "Trade Marks Registry, India — s.18(4) Trade Marks Act 1999, Rule 33 Trade Marks Rules 2017",
  "description": "Response to an Examination Report issued by the Trade Marks Registry. Matter details are pre-filled; the attorney supplies the submissions and the grounds relied upon.",
  "fields": [
    {
      "key": "FIRM_NAME",
      "label": "Firm name",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "FIRM_ADDRESS",
      "label": "Firm address",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "FIRM_CONTACT",
      "label": "Firm contact line",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "ATTORNEY_NAME",
      "label": "Signing attorney",
      "kind": {
        "type": "text",
        "maxLength": 120
      },
      "autofill": "matter.responsibleAttorney",
      "help": "The agent of record who signs the reply."
    },
    {
      "key": "REPLY_DATE",
      "label": "Date of this reply",
      "kind": {
        "type": "date",
        "notBefore": "EXAM_REPORT_DATE"
      },
      "help": "Cannot be earlier than the Examination Report."
    },
    {
      "key": "REGISTRY_OFFICE",
      "label": "Registry office",
      "kind": {
        "type": "select",
        "options": [
          {
            "value": "Delhi",
            "label": "Delhi"
          },
          {
            "value": "Mumbai",
            "label": "Mumbai"
          },
          {
            "value": "Kolkata",
            "label": "Kolkata"
          },
          {
            "value": "Chennai",
            "label": "Chennai"
          },
          {
            "value": "Ahmedabad",
            "label": "Ahmedabad"
          }
        ]
      },
      "autofill": "matter.forum"
    },
    {
      "key": "TM_NUMBER",
      "label": "Application number",
      "kind": {
        "type": "digits",
        "length": 7
      },
      "autofill": "ipAsset.applicationNumber",
      "help": "Seven digits, as allotted by the Registry."
    },
    {
      "key": "TM_MARK",
      "label": "Trade mark",
      "kind": {
        "type": "text",
        "maxLength": 200
      },
      "autofill": "ipAsset.title"
    },
    {
      "key": "TM_CLASS",
      "label": "Class",
      "kind": {
        "type": "text",
        "maxLength": 60
      },
      "autofill": "ipAsset.classes",
      "help": "Nice classification, e.g. 3 or 9, 42."
    },
    {
      "key": "APPLICANT_NAME",
      "label": "Applicant",
      "kind": {
        "type": "text",
        "maxLength": 200
      },
      "autofill": "client.name"
    },
    {
      "key": "EXAM_REPORT_DATE",
      "label": "Examination Report dated",
      "kind": {
        "type": "date"
      }
    },
    {
      "key": "SUBMISSIONS",
      "label": "Submissions",
      "kind": {
        "type": "multiline",
        "maxWords": 1500
      },
      "help": "The substance of the reply. The standard opening and prayer are added by the template."
    },
    {
      "key": "GROUNDS",
      "inputOnly": true,
      "label": "Principal ground relied upon",
      "kind": {
        "type": "select",
        "options": [
          {
            "value": "PriorUse",
            "label": "Prior use and acquired distinctiveness"
          },
          {
            "value": "NoConfusion",
            "label": "No likelihood of confusion"
          },
          {
            "value": "Distinctive",
            "label": "Inherently distinctive"
          },
          {
            "value": "Consent",
            "label": "Consent / coexistence on record"
          }
        ]
      }
    },
    {
      "key": "GROUNDS_TEXT",
      "label": "Grounds",
      "kind": {
        "type": "multiline",
        "maxWords": 1000
      }
    },
    {
      "key": "FIRST_USE_DATE",
      "inputOnly": true,
      "label": "Date of first use",
      "kind": {
        "type": "date"
      },
      "shownWhen": {
        "field": "GROUNDS",
        "equals": [
          "PriorUse"
        ]
      },
      "help": "Required only where prior use is relied upon."
    },
    {
      "key": "PRIOR_USE_BLOCK",
      "label": "Prior use paragraph",
      "kind": {
        "type": "computed"
      }
    }
  ]
};

const INVOICE_MANIFEST: any = {
  "id": "invoice",
  "name": "Tax Invoice",
  "category": "Billing",
  "version": 2,
  "revised": "2026-08-10",
  "authority": "CGST Act 2017, Rule 46 — tax invoice particulars",
  "description": "GST-compliant invoice for legal services under SAC 998212. Every field is assembled by Keel from the invoice record; nothing here is typed by hand.",
  "fields": [
    {
      "key": "INVOICE_ID",
      "label": "Invoice number",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "INVOICE_DATE",
      "label": "Invoice date",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "DUE_DATE",
      "label": "Due date",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "FIRM_NAME",
      "label": "Firm name",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "FIRM_GSTIN",
      "label": "Firm GSTIN",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "FIRM_ADDRESS",
      "label": "Firm address",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "FIRM_PAN",
      "label": "Firm PAN",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "FIRM_BANK",
      "label": "Remittance details",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "CLIENT_NAME",
      "label": "Client name",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "CLIENT_GSTIN",
      "label": "Client GSTIN",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "CLIENT_ADDRESS",
      "label": "Client address",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "LINE_ITEMS_TABLE",
      "label": "Line items",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "SUBTOTAL",
      "label": "Subtotal",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "CGST_AMOUNT",
      "label": "CGST",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "SGST_AMOUNT",
      "label": "SGST",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "IGST_AMOUNT",
      "label": "IGST",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "TOTAL",
      "label": "Total",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "AMOUNT_IN_WORDS",
      "label": "Amount in words",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "SAC_CODE",
      "label": "SAC code",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "GST_TYPE",
      "label": "GST treatment",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "NOTES",
      "label": "Notes",
      "kind": {
        "type": "computed"
      }
    }
  ]
};

// A genuinely compiled examination reply, so the preview pane shows a
// document rather than a grey rectangle.
const PREVIEW_PDF_BASE64 = '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';


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


  // ---- Drafting (M9.8) --------------------------------------------------
  // The manifests are the real shipped files, so the form here is the form an
  // attorney gets. The preview is a real compiled PDF — a placeholder image
  // would hide exactly the thing the split screen exists to show.
  list_templates: () => TEMPLATES.map(withDefaults),
  get_template: (args: any) =>
    withDefaults(TEMPLATES.find(t => t.id === args?.id) ?? TM_REPLY_MANIFEST),

  // Attaching a file. Keel reads it, checks the type and strips its metadata;
  // here the point is only that the row gets a name and an id to refer to.
  stage_annexure: (args: any) => ({
    id: `staged-${(stagedCount += 1)}`,
    filename: String(args?.path ?? '').split('/').pop() || 'attachment.pdf',
    sizeBytes: 184_320,
  }),
  discard_annexure: () => undefined,

  render_document: (args: any) => {
    // Mirror Keel: required fields are checked before anything is rendered.
    const manifest = TEMPLATES
      .find(t => t.id === args?.input?.templateId) ?? TM_REPLY_MANIFEST;
    const values = args?.input?.values ?? {};
    const fieldErrors = manifest.fields
      .filter((f: any) => f.required !== false && f.kind.type !== 'computed' && !f.inputOnly)
      .filter((f: any) => !String(values[f.key] ?? '').trim())
      .map((f: any) => ({ key: f.key, label: f.label, message: `${f.label} is required.` }));

    if (fieldErrors.length > 0) {
      return { pdfBase64: null, fieldErrors, problem: null, documentId: null };
    }
    return {
      pdfBase64: PREVIEW_PDF_BASE64,
      fieldErrors: [],
      problem: null,
      documentId: args?.input?.mode === 'final' ? 'doc-generated-1' : null,
      // Keel allocates the marks from the order of the list; Deck shows what
      // came back rather than working them out again.
      annexureMarks: (args?.input?.annexures ?? []).map(
        (a: any, i: number) => ({ stagedId: a.stagedId, mark: LETTERS[i] ?? '?' }),
      ),
    };
  },

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

const LETTERS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('');
let stagedCount = 0;

const LEGAL_NOTICE_MANIFEST: any = {
  "id": "legal-notice",
  "name": "Legal Notice",
  "category": "Litigation",
  "version": 1,
  "revised": "2026-08-11",
  "authority": "Persistas & Partners house format",
  "description": "Demand notice on the firm's letterhead. The numbered sections, payment particulars and annexures are assembled by Persist; the parties, subject and dates are entered here.",
  "fields": [
    {
      "key": "PARTNER_ONE_NAME",
      "label": "Partner One Name",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "PARTNER_ONE_ROLE",
      "label": "Partner One Role",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "PARTNER_ONE_PHONE",
      "label": "Partner One Phone",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "PARTNER_ONE_EMAIL",
      "label": "Partner One Email",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "PARTNER_TWO_NAME",
      "label": "Partner Two Name",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "PARTNER_TWO_ROLE",
      "label": "Partner Two Role",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "PARTNER_TWO_PHONE",
      "label": "Partner Two Phone",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "PARTNER_TWO_EMAIL",
      "label": "Partner Two Email",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "FIRM_WEBSITE",
      "label": "Firm Website",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "FIRM_CONTACT",
      "label": "Firm Contact",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "FIRM_OFFICE_LINE_ONE",
      "label": "Firm Office Line One",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "FIRM_OFFICE_LINE_TWO",
      "label": "Firm Office Line Two",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "NOTICE_DATE",
      "label": "Date of notice",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "RECIPIENT_NAME",
      "label": "Addressee",
      "kind": {
        "type": "text",
        "maxLength": 200
      }
    },
    {
      "key": "RECIPIENT_ADDRESS_BLOCK",
      "label": "Recipient Address Block",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "MODE_OF_SERVICE",
      "label": "Mode of service",
      "kind": {
        "type": "select",
        "options": [
          {
            "value": "THROUGH SPEED POST",
            "label": "Speed post"
          },
          {
            "value": "THROUGH SPEED POST/ WHATSAPP",
            "label": "Speed post and WhatsApp"
          },
          {
            "value": "THROUGH SPEED POST/ EMAIL",
            "label": "Speed post and email"
          },
          {
            "value": "THROUGH EMAIL",
            "label": "Email"
          }
        ]
      },
      "help": "Printed under the notice type, as served."
    },
    {
      "key": "SUBJECT",
      "label": "Subject",
      "kind": {
        "type": "multiline",
        "maxWords": 120
      },
      "help": "Set in capitals on the notice. State the demand and the amount."
    },
    {
      "key": "SALUTATION",
      "label": "Salutation",
      "kind": {
        "type": "select",
        "options": [
          {
            "value": "Sir",
            "label": "Sir"
          },
          {
            "value": "Madam",
            "label": "Madam"
          },
          {
            "value": "Sir/Madam",
            "label": "Sir/Madam"
          }
        ]
      }
    },
    {
      "key": "CLIENT_NAME",
      "label": "Client",
      "kind": {
        "type": "text",
        "maxLength": 200
      },
      "autofill": "client.name"
    },
    {
      "key": "CLIENT_DESCRIPTION",
      "label": "Client's parentage",
      "kind": {
        "type": "text",
        "maxLength": 200
      },
      "required": false,
      "help": "As it appears on the notice, e.g. 'son of P. Prabhakaran'."
    },
    {
      "key": "CLIENT_ADDRESS",
      "label": "Client's address",
      "kind": {
        "type": "multiline",
        "maxLength": 400
      }
    },
    {
      "key": "SECTIONS_BLOCK",
      "label": "Sections Block",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "SIGNATORY_BLOCK",
      "label": "Signatory Block",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "ANNEXURES_BLOCK",
      "label": "Annexures Block",
      "kind": {
        "type": "computed"
      }
    },
    {
      "key": "ANNEXURE_PAGES",
      "label": "Annexure Pages",
      "kind": {
        "type": "computed"
      }
    }
  ]
};

TEMPLATES.push(LEGAL_NOTICE_MANIFEST, TM_REPLY_MANIFEST, INVOICE_MANIFEST);

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
