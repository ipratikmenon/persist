// THE ONLY AI ENTRY POINT FROM DECK.
#![allow(dead_code)]
// Deck calls invoke('ai_request', ...) — this command routes to ai_router.rs.
// Deck never knows which model ran. Deck never calls Anthropic directly.
//
// Full implementation: Phase 3 Module 22.
// Stub: accepts call, returns placeholder response.

use crate::AppState;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AIResponse {
    pub content: String,
    pub model_used: String,   // returned for logging — Deck must NOT show this to users
    pub task_type: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AIRequestParams {
    pub task_type: String,
    pub context: String,
    pub prompt: String,
    pub deep_analysis: bool,
}

#[tauri::command]
pub async fn ai_request(
    task_type: String,
    context: String,
    prompt: String,
    deep_analysis: bool,
    _state: tauri::State<'_, AppState>,
) -> Result<AIResponse, String> {
    // STUB — Phase 3 replaces this with services::ai_router::route(...)
    let _ = (context, deep_analysis);
    Ok(AIResponse {
        content: format!("STUB response for task_type={task_type}, prompt={prompt}"),
        model_used: "stub".into(),
        task_type,
    })
}
