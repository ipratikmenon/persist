// AI Router — the ONLY place in Keel that calls the Anthropic API.
// Full implementation: Phase 3 Module 22.
//
// Routing tiers (from config/ai_thresholds.rs):
//   Haiku  → ThreadSummary, SmartReplyChips, EmailAutoTag, SentinelEval, MetadataExtract, FormAutoFill
//   Sonnet → PersistChat, AIDrafting, SmartForm, DocComparison, DailyBrief, ContractExtraction
//   Opus   → PatentClaims, DiligenceReport, explicit attorney deep_analysis request

// STUB — Phase 3 replaces this with full three-tier routing logic.
#![allow(dead_code)]

use crate::config::ai_thresholds::{ModelTier, TASK_ROUTING};

pub struct TaskRequest {
    pub task_type: String,
    pub context: String,
    pub prompt: String,
    pub deep_analysis: bool,
}

pub struct RouteResult {
    pub model: String,
    pub tier: ModelTier,
}

pub fn select_model(task_type: &str, deep_analysis: bool) -> RouteResult {
    if deep_analysis {
        return RouteResult {
            model: "claude-opus-4-6".into(),
            tier: ModelTier::Opus,
        };
    }

    let tier = TASK_ROUTING
        .iter()
        .find(|(t, _)| *t == task_type)
        .map(|(_, tier)| tier.clone())
        .unwrap_or(ModelTier::Sonnet);

    let model = match tier {
        ModelTier::Haiku => "claude-haiku-4-5-20251001",
        ModelTier::Sonnet => "claude-sonnet-4-6",
        ModelTier::Opus => "claude-opus-4-6",
    };

    RouteResult { model: model.into(), tier }
}
