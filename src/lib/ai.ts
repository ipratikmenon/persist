// THE ONLY AI ENTRY POINT FROM DECK.
// All AI calls from any component must go through this function.
// Deck never knows which model ran. Never import Anthropic SDK here.
// The routing logic lives in src-tauri/src/services/ai_router.rs.

import { invoke } from '@tauri-apps/api/core';
import type { AIRequestParams, AIResponse } from './ipc-types';

export async function aiRequest(params: AIRequestParams): Promise<AIResponse> {
  return invoke<AIResponse>('ai_request', {
    taskType: params.taskType,
    context: params.context,
    prompt: params.prompt,
    deepAnalysis: params.deepAnalysis ?? false,
  });
  // Keel decides: Haiku / Sonnet / Opus.
  // Deck never sees modelUsed in the UI.
}
