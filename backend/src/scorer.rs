use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Complexity tier produced by the 15-dimension scorer.
/// This is DISTINCT from `config::Tier` which represents provider pricing tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplexityTier {
    Simple,
    Medium,
    Complex,
    Reasoning,
}

/// Full scoring result for observability / logging.
#[derive(Debug, Clone, Serialize)]
pub struct ScoringResult {
    pub tier: ComplexityTier,
    pub raw_score: f64,
    pub confidence: f64,
    pub signals: Vec<String>,
    pub override_applied: Option<String>,
    /// Number of agentic keywords detected in the message text.
    pub agentic_keyword_count: usize,
}

/// Per-dimension raw scores (before weighting).
#[derive(Debug, Clone, Default, Serialize)]
pub struct DimensionScores {
    pub token_count: f64,
    pub code_presence: f64,
    pub reasoning_markers: f64,
    pub technical_terms: f64,
    pub creative_markers: f64,
    pub simple_indicators: f64,
    pub multi_step_patterns: f64,
    pub question_complexity: f64,
    pub imperative_verbs: f64,
    pub constraint_count: f64,
    pub output_format: f64,
    pub reference_complexity: f64,
    pub negation_complexity: f64,
    pub domain_specificity: f64,
    pub agentic_task: f64,
}

// ---------------------------------------------------------------------------
// Configuration structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScorerConfig {
    pub enabled: bool,
    pub weights: ScorerWeights,
    pub tier_boundaries: TierBoundaries,
    pub token_thresholds: TokenThresholds,
    pub confidence_steepness: f64,
    pub confidence_threshold: f64,
    pub max_tokens_force_complex: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScorerWeights {
    pub token_count: f64,
    pub code_presence: f64,
    pub reasoning_markers: f64,
    pub technical_terms: f64,
    pub creative_markers: f64,
    pub simple_indicators: f64,
    pub multi_step_patterns: f64,
    pub question_complexity: f64,
    pub imperative_verbs: f64,
    pub constraint_count: f64,
    pub output_format: f64,
    pub reference_complexity: f64,
    pub negation_complexity: f64,
    pub domain_specificity: f64,
    pub agentic_task: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierBoundaries {
    pub simple_upper: f64,
    pub medium_upper: f64,
    pub complex_upper: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenThresholds {
    pub short_upper: usize,
    pub long_lower: usize,
}

// ---------------------------------------------------------------------------
// Defaults (matching ClawRouter reference)
// ---------------------------------------------------------------------------

impl Default for ScorerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            weights: ScorerWeights::default(),
            tier_boundaries: TierBoundaries::default(),
            token_thresholds: TokenThresholds::default(),
            confidence_steepness: 12.0,
            confidence_threshold: 0.7,
            max_tokens_force_complex: 100_000,
        }
    }
}

impl Default for ScorerWeights {
    fn default() -> Self {
        Self {
            token_count: 0.08,
            code_presence: 0.15,
            reasoning_markers: 0.18,
            technical_terms: 0.10,
            creative_markers: 0.05,
            simple_indicators: 0.02,
            multi_step_patterns: 0.12,
            question_complexity: 0.05,
            imperative_verbs: 0.03,
            constraint_count: 0.04,
            output_format: 0.03,
            reference_complexity: 0.02,
            negation_complexity: 0.01,
            domain_specificity: 0.02,
            agentic_task: 0.04,
        }
    }
}

impl Default for TierBoundaries {
    fn default() -> Self {
        Self {
            simple_upper: 0.0,
            medium_upper: 0.3,
            complex_upper: 0.5,
        }
    }
}

impl Default for TokenThresholds {
    fn default() -> Self {
        Self {
            short_upper: 500,
            long_lower: 3000,
        }
    }
}

// ---------------------------------------------------------------------------
// Keyword lists (English only)
// ---------------------------------------------------------------------------

const CODE_KEYWORDS: &[&str] = &[
    "function", "class", "import", "const", "let", "var", "return",
    "async", "await", "def ", "print(", "console.log", "```",
    "pub fn", "impl ", "struct ", "enum ", "SELECT", "INSERT",
    "UPDATE", "DELETE", "CREATE TABLE",
];

const REASONING_KEYWORDS: &[&str] = &[
    "prove", "theorem", "derive", "step by step", "chain of thought",
    "formally", "mathematical", "proof", "logically", "contradiction",
    "induction", "hypothesis", "therefore", "axiom", "lemma",
    "corollary", "deduce", "implies",
];

const TECHNICAL_KEYWORDS: &[&str] = &[
    "algorithm", "optimize", "architecture", "distributed", "kubernetes",
    "microservice", "database", "infrastructure", "concurrent", "latency",
    "throughput", "scalable", "middleware", "authentication",
    "authorization", "encryption",
];

const CREATIVE_KEYWORDS: &[&str] = &[
    "story", "poem", "compose", "brainstorm", "creative", "imagine",
    "write a", "fiction", "narrative", "character", "plot", "metaphor",
];

const SIMPLE_KEYWORDS: &[&str] = &[
    "what is", "define", "translate", "hello", "yes or no",
    "capital of", "how old", "who is", "when was", "meaning of",
    "true or false",
];

const IMPERATIVE_KEYWORDS: &[&str] = &[
    "build", "create", "implement", "design", "develop", "construct",
    "generate", "deploy", "configure", "set up", "refactor", "migrate",
    "integrate",
];

const CONSTRAINT_KEYWORDS: &[&str] = &[
    "under", "at most", "at least", "within", "no more than",
    "o(", "maximum", "minimum", "limit", "budget", "constraint",
];

const OUTPUT_FORMAT_KEYWORDS: &[&str] = &[
    "json", "yaml", "xml", "table", "csv", "markdown", "schema",
    "format as", "structured", "output as",
];

const REFERENCE_KEYWORDS: &[&str] = &[
    "above", "below", "previous", "following", "the docs", "the api",
    "the code", "earlier", "attached", "mentioned",
];

const NEGATION_KEYWORDS: &[&str] = &[
    "don't", "do not", "avoid", "never", "without", "except",
    "exclude", "no longer", "must not", "shouldn't",
];

const DOMAIN_KEYWORDS: &[&str] = &[
    "quantum", "fpga", "vlsi", "risc-v", "asic", "photonics",
    "genomics", "proteomics", "topological", "homomorphic",
    "zero-knowledge", "lattice-based",
];

const AGENTIC_KEYWORDS: &[&str] = &[
    "read file", "read the file", "look at", "check the", "open the",
    "edit", "modify", "update the", "change the", "write to",
    "create file", "execute", "deploy", "install", "npm", "pip",
    "compile", "after that", "and also", "once done", "step 1",
    "step 2", "fix", "debug", "until it works", "keep trying",
    "iterate", "make sure", "verify", "confirm",
];

// ---------------------------------------------------------------------------
// Lazy-compiled regex for multi-step detection
// ---------------------------------------------------------------------------

static MULTI_STEP_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(first\b.*\bthen\b|step\s+\d|1\.\s.*2\.\s)").unwrap()
});

// ---------------------------------------------------------------------------
// Scorer
// ---------------------------------------------------------------------------

pub struct Scorer;

impl Scorer {
    /// Score a chat completion request's messages and return a complexity tier.
    pub fn score(messages: &[Value], config: &ScorerConfig) -> ScoringResult {
        let text = extract_text(messages);
        let estimated_tokens = estimate_token_count(&text);
        let mut signals: Vec<String> = Vec::new();

        let (agentic_score, agentic_keyword_count) = score_agentic_task(&text, AGENTIC_KEYWORDS, &mut signals);

        // --- Score each dimension ---
        let dimensions = DimensionScores {
            token_count: score_token_count(&text, &config.token_thresholds),
            code_presence: score_keyword_match(&text, CODE_KEYWORDS, "code", &mut signals),
            reasoning_markers: score_keyword_match(
                &text,
                REASONING_KEYWORDS,
                "reasoning",
                &mut signals,
            ),
            technical_terms: score_keyword_match(
                &text,
                TECHNICAL_KEYWORDS,
                "technical",
                &mut signals,
            ),
            creative_markers: score_keyword_match(
                &text,
                CREATIVE_KEYWORDS,
                "creative",
                &mut signals,
            ),
            simple_indicators: score_keyword_match(
                &text,
                SIMPLE_KEYWORDS,
                "simple",
                &mut signals,
            ),
            multi_step_patterns: score_multi_step(&text, &mut signals),
            question_complexity: score_question_complexity(&text, &mut signals),
            imperative_verbs: score_keyword_match(
                &text,
                IMPERATIVE_KEYWORDS,
                "imperative",
                &mut signals,
            ),
            constraint_count: score_keyword_match(
                &text,
                CONSTRAINT_KEYWORDS,
                "constraint",
                &mut signals,
            ),
            output_format: score_keyword_match(
                &text,
                OUTPUT_FORMAT_KEYWORDS,
                "output_format",
                &mut signals,
            ),
            reference_complexity: score_keyword_match(
                &text,
                REFERENCE_KEYWORDS,
                "reference",
                &mut signals,
            ),
            negation_complexity: score_keyword_match(
                &text,
                NEGATION_KEYWORDS,
                "negation",
                &mut signals,
            ),
            domain_specificity: score_keyword_match(
                &text,
                DOMAIN_KEYWORDS,
                "domain",
                &mut signals,
            ),
            agentic_task: agentic_score,
        };

        // --- Weighted sum ---
        let raw_score = compute_weighted_score(&dimensions, &config.weights);

        // --- Classify tier ---
        let mut tier = classify_tier(raw_score, &config.tier_boundaries);

        // --- Confidence ---
        let confidence = calibrate_confidence(raw_score, &config.tier_boundaries, config.confidence_steepness);

        // --- Apply overrides ---
        let mut override_applied: Option<String> = None;

        // Override: token count > threshold → force Complex
        if estimated_tokens > config.max_tokens_force_complex {
            if (tier as u8) < (ComplexityTier::Complex as u8) {
                tier = ComplexityTier::Complex;
                override_applied = Some("token_count_force_complex".to_string());
            }
        }

        // Override: structured output detected → minimum Medium
        if dimensions.output_format > 0.0 && tier == ComplexityTier::Simple {
            tier = ComplexityTier::Medium;
            override_applied = Some("structured_output_min_medium".to_string());
        }

        // Override: ≥2 reasoning markers → force Reasoning
        // (score >= 0.6 corresponds to 2+ keyword matches)
        if dimensions.reasoning_markers >= 0.6 {
            tier = ComplexityTier::Reasoning;
            override_applied = Some("reasoning_markers_force".to_string());
        }

        ScoringResult {
            tier,
            raw_score,
            confidence,
            signals,
            override_applied,
            agentic_keyword_count,
        }
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Extract user-message text from the messages array.
/// Handles both string and array-style content.
fn extract_text(messages: &[Value]) -> String {
    let mut parts: Vec<String> = Vec::new();

    for msg in messages {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
        if role != "user" {
            continue;
        }

        if let Some(content) = msg.get("content") {
            match content {
                Value::String(s) => parts.push(s.clone()),
                Value::Array(arr) => {
                    for item in arr {
                        if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                                parts.push(text.to_string());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    parts.join("\n").to_lowercase()
}

/// Rough token count estimate: ~4 chars per token.
fn estimate_token_count(text: &str) -> usize {
    text.len() / 4
}

/// Score based on estimated token count.
fn score_token_count(text: &str, thresholds: &TokenThresholds) -> f64 {
    let tokens = estimate_token_count(text);
    if tokens < thresholds.short_upper {
        -1.0
    } else if tokens > thresholds.long_lower {
        1.0
    } else {
        0.0
    }
}

/// Count keyword matches and map to a score: 0→0.0, 1→0.3, 2→0.6, 3+→1.0.
fn score_keyword_match(
    text: &str,
    keywords: &[&str],
    signal_name: &str,
    signals: &mut Vec<String>,
) -> f64 {
    let count = keywords.iter().filter(|kw| text.contains(**kw)).count();
    if count > 0 {
        signals.push(format!("{}:{}", signal_name, count));
    }
    match count {
        0 => 0.0,
        1 => 0.3,
        2 => 0.6,
        _ => 1.0,
    }
}

/// Detect multi-step patterns via regex.
fn score_multi_step(text: &str, signals: &mut Vec<String>) -> f64 {
    if MULTI_STEP_RE.is_match(text) {
        signals.push("multi_step".to_string());
        0.5
    } else {
        0.0
    }
}

/// Count question marks — more than 3 indicates higher complexity.
fn score_question_complexity(text: &str, signals: &mut Vec<String>) -> f64 {
    let count = text.chars().filter(|c| *c == '?').count();
    if count > 3 {
        signals.push(format!("questions:{}", count));
        0.5
    } else {
        0.0
    }
}

/// Tiered agentic task scoring: 0→0.0, 1-2→0.2, 3→0.6, 4+→1.0.
/// Returns (score, raw_keyword_count).
fn score_agentic_task(
    text: &str,
    keywords: &[&str],
    signals: &mut Vec<String>,
) -> (f64, usize) {
    let count = keywords.iter().filter(|kw| text.contains(**kw)).count();
    let score = match count {
        0 => 0.0,
        1..=2 => 0.2,
        3 => 0.6,
        _ => 1.0,
    };
    if count > 0 {
        signals.push(format!("agentic:{}", count));
    }
    (score, count)
}

/// Weighted sum of all dimensions.
/// Note: simple_indicators weight is subtracted (negative contribution).
fn compute_weighted_score(d: &DimensionScores, w: &ScorerWeights) -> f64 {
    d.token_count * w.token_count
        + d.code_presence * w.code_presence
        + d.reasoning_markers * w.reasoning_markers
        + d.technical_terms * w.technical_terms
        + d.creative_markers * w.creative_markers
        - d.simple_indicators * w.simple_indicators // NEGATIVE
        + d.multi_step_patterns * w.multi_step_patterns
        + d.question_complexity * w.question_complexity
        + d.imperative_verbs * w.imperative_verbs
        + d.constraint_count * w.constraint_count
        + d.output_format * w.output_format
        + d.reference_complexity * w.reference_complexity
        + d.negation_complexity * w.negation_complexity
        + d.domain_specificity * w.domain_specificity
        + d.agentic_task * w.agentic_task
}

/// Map raw score to complexity tier.
fn classify_tier(score: f64, boundaries: &TierBoundaries) -> ComplexityTier {
    if score < boundaries.simple_upper {
        ComplexityTier::Simple
    } else if score < boundaries.medium_upper {
        ComplexityTier::Medium
    } else if score < boundaries.complex_upper {
        ComplexityTier::Complex
    } else {
        ComplexityTier::Reasoning
    }
}

/// Sigmoid confidence calibration based on distance from nearest tier boundary.
/// Returns a value in [0.5, 1.0].
fn calibrate_confidence(score: f64, boundaries: &TierBoundaries, steepness: f64) -> f64 {
    let boundary_points = [
        boundaries.simple_upper,
        boundaries.medium_upper,
        boundaries.complex_upper,
    ];
    let min_distance = boundary_points
        .iter()
        .map(|b| (score - b).abs())
        .fold(f64::MAX, f64::min);
    1.0 / (1.0 + (-steepness * min_distance).exp())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_user_message(content: &str) -> Value {
        serde_json::json!({
            "role": "user",
            "content": content
        })
    }

    fn score_text(text: &str) -> ScoringResult {
        let messages = vec![make_user_message(text)];
        Scorer::score(&messages, &ScorerConfig::default())
    }

    #[test]
    fn test_simple_query() {
        let result = score_text("What is Rust?");
        assert_eq!(result.tier, ComplexityTier::Simple);
        assert!(result.raw_score < 0.0);
    }

    #[test]
    fn test_code_query() {
        let result = score_text(
            "Write a function that implements a class with async/await \
             and uses import statements. Include a struct definition.",
        );
        // Should score high on code_presence (4+ keywords) and imperative
        assert!(result.raw_score > 0.0);
        assert!(
            result.tier == ComplexityTier::Medium
                || result.tier == ComplexityTier::Complex
        );
    }

    #[test]
    fn test_reasoning_override() {
        let result = score_text(
            "Prove the theorem using mathematical induction. \
             Derive the proof step by step using formal logic.",
        );
        // ≥2 reasoning markers should force Reasoning tier
        assert_eq!(result.tier, ComplexityTier::Reasoning);
        assert_eq!(
            result.override_applied,
            Some("reasoning_markers_force".to_string())
        );
    }

    #[test]
    fn test_multi_step_detection() {
        let result = score_text(
            "First, set up the database schema, then create the API endpoints, \
             and deploy the microservice to kubernetes.",
        );
        assert!(result.signals.iter().any(|s| s == "multi_step"));
        assert!(result.raw_score > 0.0);
    }

    #[test]
    fn test_question_complexity() {
        let result = score_text(
            "What is the algorithm? How does it optimize? \
             Why is it distributed? When should I use it? \
             Where does latency come from?",
        );
        // 5 question marks → should trigger question_complexity
        assert!(result.signals.iter().any(|s| s.starts_with("questions:")));
    }

    #[test]
    fn test_agentic_task() {
        let result = score_text(
            "Read the file, edit the code, fix the bug, \
             deploy it, and make sure it works. After that, verify.",
        );
        assert!(result.signals.iter().any(|s| s.starts_with("agentic:")));
    }

    #[test]
    fn test_structured_output_override() {
        let result = score_text("What is json?");
        // "what is" triggers simple, "json" triggers output_format
        // Simple + output_format > 0 → override to Medium
        assert_eq!(result.tier, ComplexityTier::Medium);
        assert_eq!(
            result.override_applied,
            Some("structured_output_min_medium".to_string())
        );
    }

    #[test]
    fn test_extract_text_string_content() {
        let messages = vec![serde_json::json!({
            "role": "user",
            "content": "Hello World"
        })];
        let text = extract_text(&messages);
        assert_eq!(text, "hello world");
    }

    #[test]
    fn test_extract_text_array_content() {
        let messages = vec![serde_json::json!({
            "role": "user",
            "content": [
                {"type": "text", "text": "Hello"},
                {"type": "image_url", "image_url": {"url": "data:..."}}
            ]
        })];
        let text = extract_text(&messages);
        assert_eq!(text, "hello");
    }

    #[test]
    fn test_extract_text_skips_non_user() {
        let messages = vec![
            serde_json::json!({"role": "system", "content": "You are helpful."}),
            serde_json::json!({"role": "user", "content": "Hi there"}),
            serde_json::json!({"role": "assistant", "content": "Hello!"}),
        ];
        let text = extract_text(&messages);
        assert_eq!(text, "hi there");
    }

    #[test]
    fn test_confidence_high_away_from_boundary() {
        let boundaries = TierBoundaries::default();
        // Score of 0.15 is between 0.0 and 0.3, distance 0.15 from each
        let conf = calibrate_confidence(0.15, &boundaries, 12.0);
        assert!(conf > 0.5);
        assert!(conf < 1.0);
    }

    #[test]
    fn test_confidence_low_at_boundary() {
        let boundaries = TierBoundaries::default();
        // Score exactly at boundary 0.3
        let conf = calibrate_confidence(0.3, &boundaries, 12.0);
        // Distance is 0.0, sigmoid(0) = 0.5
        assert!((conf - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_domain_specific() {
        let result = score_text(
            "Explain quantum computing and homomorphic encryption \
             for lattice-based cryptography.",
        );
        assert!(result.signals.iter().any(|s| s.starts_with("domain:")));
    }

    #[test]
    fn test_empty_messages() {
        let result = Scorer::score(&[], &ScorerConfig::default());
        assert_eq!(result.tier, ComplexityTier::Simple);
    }
}
