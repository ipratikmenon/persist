import { create } from 'zustand';
import type { SyncStatus } from '@/lib/ipc-types';

interface SyncStore {
  status: SyncStatus;
  setStatus: (status: SyncStatus) => void;
}

export const useSyncStore = create<SyncStore>((set) => ({
  status: {
    lastSyncedAt: null,
    isSyncing: false,
    pendingChanges: 0,
  },
  setStatus: (status) => set({ status }),
}));
