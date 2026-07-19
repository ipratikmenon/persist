// Mail store — Phase 4 Module 15.
// Stub: minimal state for the module to compile.

import { create } from 'zustand';

export type MailTab = 'focused' | 'other' | 'done' | 'snoozed';

interface MailStore {
  activeTab: MailTab;
  activeThreadId: string | null;
  setActiveTab: (tab: MailTab) => void;
  setActiveThread: (id: string | null) => void;
}

export const useMailStore = create<MailStore>((set) => ({
  activeTab: 'focused',
  activeThreadId: null,
  setActiveTab: (activeTab) => set({ activeTab }),
  setActiveThread: (activeThreadId) => set({ activeThreadId }),
}));
