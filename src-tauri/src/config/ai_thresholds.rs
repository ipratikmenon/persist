// AI routing thresholds and task-to-model mapping.
// Adjust these constants to tune cost vs quality without touching routing logic.
#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq)]
pub enum ModelTier {
    Haiku,
    Sonnet,
    Opus,
}

// Maximum context tokens before Haiku is ruled out (Haiku ceiling)
pub const HAIKU_MAX_CONTEXT_TOKENS: usize = 180_000;

// Sonnet confidence threshold — below this, escalate to Opus
// Range 0.0–1.0; used in Phase 3 escalation loop
pub const SONNET_CONFIDENCE_THRESHOLD: f32 = 0.75;

// Task type → default model tier.
// deep_analysis=true always escalates to Opus regardless of this table.
pub const TASK_ROUTING: &[(&str, ModelTier)] = &[
    // Haiku tier — lightweight, high-frequency
    ("ThreadSummary",     ModelTier::Haiku),
    ("SmartReplyChips",   ModelTier::Haiku),
    ("EmailAutoTag",      ModelTier::Haiku),
    ("SentinelEval",      ModelTier::Haiku),
    ("MetadataExtract",   ModelTier::Haiku),
    ("FormAutoFill",      ModelTier::Haiku),

    // Sonnet tier — daily attorney work
    ("PersistChat",           ModelTier::Sonnet),
    ("AIDrafting",            ModelTier::Sonnet),
    ("SmartForm",             ModelTier::Sonnet),
    ("DocComparison",         ModelTier::Sonnet),
    ("DailyBrief",            ModelTier::Sonnet),
    ("ContractExtraction",    ModelTier::Sonnet),
    ("MeetingIntelligence",   ModelTier::Sonnet),

    // Opus tier — deep analysis
    ("PatentClaims",    ModelTier::Opus),
    ("DiligenceReport", ModelTier::Opus),
];
