import { create } from 'zustand';

export type MessageRole = 'user' | 'assistant';

export interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  taskType: string;
  deepAnalysis: boolean;
  createdAt: string;
}

interface ChatStore {
  messages: ChatMessage[];
  isStreaming: boolean;
  currentMatterContext: string | null;

  // Actions
  addMessage: (msg: ChatMessage) => void;
  setStreaming: (streaming: boolean) => void;
  setMatterContext: (matterId: string | null) => void;
  clearSession: () => void;
}

export const useChatStore = create<ChatStore>((set) => ({
  messages: [],
  isStreaming: false,
  currentMatterContext: null,

  addMessage: (msg) => set((s) => ({ messages: [...s.messages, msg] })),
  setStreaming: (isStreaming) => set({ isStreaming }),
  setMatterContext: (currentMatterContext) => set({ currentMatterContext }),
  clearSession: () => set({ messages: [], isStreaming: false }),
}));
