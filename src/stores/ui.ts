import { create } from 'zustand';

export type ActiveModule =
  | 'matters'
  | 'dockets'
  | 'documents'
  | 'billing'
  | 'mail'
  | 'analytics'
  | 'settings';

interface UIStore {
  // Navigation
  activeModule: ActiveModule;
  sidebarCollapsed: boolean;
  activeMatterId: string | null;
  activeTab: string;

  // Overlays
  chatOpen: boolean;
  commandPaletteOpen: boolean;

  // Actions
  setActiveModule: (module: ActiveModule) => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  setActiveMatterId: (id: string | null) => void;
  setActiveTab: (tab: string) => void;
  openChat: () => void;
  closeChat: () => void;
  toggleChat: () => void;
  openCommandPalette: () => void;
  closeCommandPalette: () => void;
}

export const useUIStore = create<UIStore>((set) => ({
  activeModule: 'matters',
  sidebarCollapsed: false,
  activeMatterId: null,
  activeTab: 'overview',
  chatOpen: false,
  commandPaletteOpen: false,

  setActiveModule: (activeModule) => set({ activeModule }),
  setSidebarCollapsed: (sidebarCollapsed) => set({ sidebarCollapsed }),
  setActiveMatterId: (activeMatterId) => set({ activeMatterId }),
  setActiveTab: (activeTab) => set({ activeTab }),
  openChat: () => set({ chatOpen: true }),
  closeChat: () => set({ chatOpen: false }),
  toggleChat: () => set((s) => ({ chatOpen: !s.chatOpen })),
  openCommandPalette: () => set({ commandPaletteOpen: true }),
  closeCommandPalette: () => set({ commandPaletteOpen: false }),
}));
