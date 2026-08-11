// The only place the portal talks to the backend.
//
// TOKEN HANDLING — deliberately not localStorage
//
// The access token lives in a module variable and nowhere else. Putting it in
// localStorage or sessionStorage would mean any script that manages to run on
// this page can read it, and a stolen token is a client's entire matter file.
// In memory, it dies with the tab.
//
// The refresh token is an httpOnly cookie the backend sets, so this code cannot
// read it and neither can an injected script. On a cold load there is no access
// token, so `restoreSession` asks the backend to mint one from the cookie —
// which is what makes a page refresh not look like a logout.
//
// A 401 on any call means the access token expired mid-session. `request`
// refreshes once and retries once. Twice would be a loop.

// The API is namespaced, in every environment.
//
// It cannot share a path space with the client routes: `/matters` and
// `/invoices` are both React routes and API endpoints, so a same-origin
// deployment serving both from `/` cannot tell a page load from an API call —
// the router would hand a browser navigation to FastAPI and the client would
// see raw JSON instead of the page.
//
// So everything the portal calls lives under /api, and the edge (Cloudflare in
// production, Vite in dev) strips the prefix before it reaches FastAPI.
const BASE = '/api';

let accessToken: string | null = null;
let refreshing: Promise<boolean> | null = null;

/** Listeners fired when the session ends, so the app can send them to login. */
const sessionEndedHandlers = new Set<() => void>();

export function onSessionEnded(handler: () => void): () => void {
  sessionEndedHandlers.add(handler);
  return () => sessionEndedHandlers.delete(handler);
}

function endSession() {
  accessToken = null;
  sessionEndedHandlers.forEach((h) => h());
}

export function isAuthenticated(): boolean {
  return accessToken !== null;
}

export class ApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
  ) {
    super(message);
  }
}

/** Ask the backend to mint a new access token from the refresh cookie. */
async function refreshAccessToken(): Promise<boolean> {
  // Concurrent 401s must not each fire their own refresh: the backend rotates
  // the token, so the second would present one already spent and be read as a
  // replay — which revokes the whole family and logs the client out.
  if (refreshing) return refreshing;

  refreshing = (async () => {
    try {
      const response = await fetch(`${BASE}/auth/refresh`, {
        method: 'POST',
        credentials: 'include',
      });
      if (!response.ok) return false;
      const body = (await response.json()) as { accessToken: string };
      accessToken = body.accessToken;
      return true;
    } catch {
      return false;
    } finally {
      refreshing = null;
    }
  })();

  return refreshing;
}

/** Restore a session on a cold load. Returns false when there is none. */
export async function restoreSession(): Promise<boolean> {
  return refreshAccessToken();
}

async function request<T>(
  path: string,
  init: RequestInit = {},
  retry = true,
): Promise<T> {
  const headers = new Headers(init.headers);
  if (accessToken) headers.set('Authorization', `Bearer ${accessToken}`);

  const response = await fetch(`${BASE}${path}`, {
    ...init,
    headers,
    credentials: 'include',
  });

  if (response.status === 401 && retry) {
    if (await refreshAccessToken()) {
      return request<T>(path, init, false);
    }
    endSession();
    throw new ApiError('Your session has ended. Please sign in again.', 401);
  }

  if (!response.ok) {
    throw new ApiError(await problemFrom(response), response.status);
  }

  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}

/** FastAPI puts its message in `detail`. Anything else gets a plain sentence. */
async function problemFrom(response: Response): Promise<string> {
  try {
    const body = await response.json();
    if (typeof body?.detail === 'string') return body.detail;
  } catch {
    // Not JSON — fall through.
  }
  return response.status === 429
    ? 'Too many attempts. Please wait a little and try again.'
    : 'Something went wrong. Please try again.';
}

// ---------------------------------------------------------------------------
// Types — mirror portal/backend/app/models.py
// ---------------------------------------------------------------------------

export interface MatterSummary {
  id: string;
  title: string;
  matterType: string;
  status: string;
  openedDate: string;
  nextDeadlineDate: string | null;
  nextDeadlineEvent: string | null;
}

export interface Deadline {
  id: string;
  matterId: string;
  docketingEvent: string;
  dueDate: string;
  status: string;
}

export interface IpAsset {
  id: string;
  matterId: string;
  assetType: string;
  title: string;
  applicationNumber: string | null;
  registrationNumber: string | null;
  status: string;
  classes: number[];
  nextRenewalDate: string | null;
}

export interface SharedDocument {
  id: string;
  matterId: string | null;
  filename: string;
  category: string | null;
  fileSizeBytes: number;
  sharedAt: string;
}

export interface MatterDetail extends MatterSummary {
  forum: string | null;
  jurisdiction: string;
  clientNotes: string | null;
  responsibleAttorney: string | null;
  deadlines: Deadline[];
  ipAssets: IpAsset[];
  documents: SharedDocument[];
}

/** Money arrives as a decimal string, not a number — see models.py. Keeping it
 *  a string all the way to the screen is what stops a float from rounding a
 *  client's bill by a paisa. */
export interface InvoiceSummary {
  id: string;
  status: string;
  invoiceDate: string;
  dueDate: string | null;
  totalWithTax: string;
  amountPaid: string;
  amountDue: string;
}

export interface Payment {
  id: string;
  amount: string;
  paymentDate: string;
  method: string;
}

export interface InvoiceDetail extends InvoiceSummary {
  subtotal: string;
  cgstAmount: string;
  sgstAmount: string;
  igstAmount: string;
  notes: string | null;
  payments: Payment[];
}

export interface Notification {
  id: string;
  kind: string;
  title: string;
  body: string | null;
  matterId: string | null;
  isRead: boolean;
  createdAt: string;
}

export interface Profile {
  id: string;
  fullName: string;
  email: string;
  phone: string | null;
  clientName: string;
}

export interface Upload {
  id: string;
  filename: string;
  fileSizeBytes: number;
  scanStatus: string;
  status: string;
  uploadedAt: string;
}

// ---------------------------------------------------------------------------
// Endpoints
// ---------------------------------------------------------------------------

export const api = {
  auth: {
    /** Always resolves. The backend answers the same way for an unknown
     *  address, so the UI must not imply otherwise. */
    async requestOtp(email: string): Promise<void> {
      await fetch(`${BASE}/auth/request-otp`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email }),
        credentials: 'include',
      }).then(async (r) => {
        if (!r.ok) throw new ApiError(await problemFrom(r), r.status);
      });
    },

    async verifyOtp(email: string, code: string): Promise<void> {
      const response = await fetch(`${BASE}/auth/verify-otp`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, code }),
        credentials: 'include',
      });
      if (!response.ok) throw new ApiError(await problemFrom(response), response.status);
      const body = (await response.json()) as { accessToken: string };
      accessToken = body.accessToken;
    },

    async logout(): Promise<void> {
      try {
        await fetch(`${BASE}/auth/logout`, { method: 'POST', credentials: 'include' });
      } finally {
        // Local state is cleared whether or not the call succeeded — a failed
        // logout must not leave someone looking signed in.
        endSession();
      }
    },
  },

  matters: {
    list: () => request<MatterSummary[]>('/matters'),
    get: (id: string) => request<MatterDetail>(`/matters/${encodeURIComponent(id)}`),
  },

  deadlines: {
    list: () => request<Deadline[]>('/deadlines'),
  },

  documents: {
    list: (matterId?: string) =>
      request<SharedDocument[]>(
        matterId ? `/documents?matter_id=${encodeURIComponent(matterId)}` : '/documents',
      ),

    /** The backend answers with a 302 to a URL that stops working in five
     *  minutes. Handing it to the browser is the whole mechanism — the portal
     *  never sees the object itself. */
    downloadUrl: (id: string) => `${BASE}/documents/${encodeURIComponent(id)}/download`,

    async upload(file: File, matterId?: string): Promise<Upload> {
      const form = new FormData();
      form.append('file', file);
      if (matterId) form.append('matter_id', matterId);
      // No Content-Type header: the browser must set the multipart boundary.
      return request<Upload>('/documents/upload', { method: 'POST', body: form });
    },

    uploads: () => request<Upload[]>('/uploads'),
  },

  invoices: {
    list: () => request<InvoiceSummary[]>('/invoices'),
    get: (id: string) => request<InvoiceDetail>(`/invoices/${encodeURIComponent(id)}`),
    pdfUrl: (id: string) => `${BASE}/invoices/${encodeURIComponent(id)}/pdf`,
    dispute: (id: string, reason: string) =>
      request<{ id: string }>(`/invoices/${encodeURIComponent(id)}/dispute`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ reason }),
      }),
  },

  notifications: {
    list: () => request<Notification[]>('/notifications'),
  },

  profile: {
    get: () => request<Profile>('/profile'),
  },
};
