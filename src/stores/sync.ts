import { create } from 'zustand';
import type { SyncStatus } from '@/lib/ipc-types';

interface SyncStore {
  status: SyncStatus;
  setStatus: (status: SyncStatus) => void;
}

export const useSyncStore = create<SyncStore>((set) => ({
  // Matches Keel's default: sync is off until a server URL is configured.
  status: {
    lastSyncedAt: null,
    isSyncing: false,
    pendingChanges: 0,
    isEnabled: false,
    serverUrl: null,
    lastError: null,
  },
  setStatus: (status) => set({ status }),
}));
