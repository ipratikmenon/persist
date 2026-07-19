import { create } from 'zustand';
import type { Matter, MatterFilter, MatterSummary } from '@/lib/ipc-types';

interface MattersStore {
  // State
  matterList: MatterSummary[];
  activeMatter: Matter | null;
  filter: MatterFilter;
  isLoading: boolean;
  error: string | null;

  // Actions
  setMatterList: (list: MatterSummary[]) => void;
  setActiveMatter: (matter: Matter | null) => void;
  setFilter: (filter: MatterFilter) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  clearActiveMatter: () => void;
}

export const useMattersStore = create<MattersStore>((set) => ({
  matterList: [],
  activeMatter: null,
  filter: {},
  isLoading: false,
  error: null,

  setMatterList: (list) => set({ matterList: list }),
  setActiveMatter: (matter) => set({ activeMatter: matter }),
  setFilter: (filter) => set({ filter }),
  setLoading: (isLoading) => set({ isLoading }),
  setError: (error) => set({ error }),
  clearActiveMatter: () => set({ activeMatter: null }),
}));
