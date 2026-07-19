import { create } from 'zustand';
import type { Session } from '@/lib/ipc-types';

interface AuthStore {
  session: Session | null;
  isChecking: boolean;   // true while get_session IPC call is in flight on boot

  setSession: (session: Session | null) => void;
  setChecking: (checking: boolean) => void;
  clearSession: () => void;
}

export const useAuthStore = create<AuthStore>((set) => ({
  session:    null,
  isChecking: true,   // optimistic: assume we need to check on first render

  setSession:   (session)   => set({ session, isChecking: false }),
  setChecking:  (isChecking) => set({ isChecking }),
  clearSession: ()           => set({ session: null, isChecking: false }),
}));
