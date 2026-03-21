//! Context passing — build prompts for verifier and planner rounds.
//!
//! Token estimation uses a simple heuristic:
//! - 1 token ~ 4 ASCII characters
//! - 1 token ~ 2 non-ASCII characters (e.g. Korean)

use crate::types::{RejectCategory, VerifyResult};

/// Bytes-per-token for ASCII text.
const ASCII_CHARS_PER_TOKEN: usize = 4;
/// Bytes-per-token for non-ASCII text (CJK, etc.).
const NON_ASCII_CHARS_PER_TOKEN: usize = 2;

/// Default maximum tokens for a single round of context.
pub const DEFAULT_MAX_TOKENS: usize = 4000;

/// Tokens to keep from the tail when truncating.
const TAIL_KEEP_TOKENS: usize = 1000;

/// Estimate the number of tokens in `text`.
///
/// Heuristic: count ASCII characters at 4-per-token and non-ASCII characters
/// at 2-per-token, then sum.
pub fn estimate_tokens(text: &str) -> usize {
    let mut ascii_chars: usize = 0;
    let mut non_ascii_chars: usize = 0;
    for ch in text.chars() {
        if ch.is_ascii() {
            ascii_chars += 1;
        } else {
            non_ascii_chars += 1;
        }
    }
    let ascii_tokens = ascii_chars.div_ceil(ASCII_CHARS_PER_TOKEN);
    let non_ascii_tokens = non_ascii_chars.div_ceil(NON_ASCII_CHARS_PER_TOKEN);
    ascii_tokens + non_ascii_tokens
}

/// Truncate `text` to fit within `max_tokens`.
///
/// If the text is already within budget, return it unchanged.
/// Otherwise, keep the last `TAIL_KEEP_TOKENS` worth of characters and
/// prepend `[truncated]`.
fn truncate_to_budget(text: &str, max_tokens: usize) -> String {
    if estimate_tokens(text) <= max_tokens {
        return text.to_string();
    }
    // Keep tail: estimate how many characters correspond to TAIL_KEEP_TOKENS.
    // Use the lower bound (ASCII ratio) so we don't overshoot.
    let keep_chars = TAIL_KEEP_TOKENS * ASCII_CHARS_PER_TOKEN;
    let tail = tail_chars(text, keep_chars);
    format!("[truncated]\n{tail}")
}

/// Return up to the last `n` chars of `text`, split on a char boundary.
fn tail_chars(text: &str, n: usize) -> &str {
    if text.len() <= n {
        return text;
    }
    // Walk forward until we're within `n` bytes of the end, respecting char
    // boundaries.
    let start = text.len() - n;
    // Find the nearest char boundary at or after `start`.
    let start = text
        .char_indices()
        .map(|(i, _)| i)
        .find(|&i| i >= start)
        .unwrap_or(text.len());
    &text[start..]
}

/// Format a single prior-round summary as a bullet point.
fn format_round_summary(round: u32, result: &VerifyResult) -> String {
    match result {
        VerifyResult::Approved { reason, confidence } => {
            format!("- Round {round}: APPROVED (confidence={confidence:.2}) — {reason}")
        }
        VerifyResult::Rejected {
            reason,
            category,
            confidence,
        } => {
            let cat = category_label(category);
            format!("- Round {round}: REJECTED [{cat}] (confidence={confidence:.2}) — {reason}")
        }
        VerifyResult::NeedsInfo { question } => {
            format!("- Round {round}: NEEDS_INFO — {question}")
        }
    }
}

/// Human-readable label for a [`RejectCategory`].
fn category_label(cat: &RejectCategory) -> &'static str {
    match cat {
        RejectCategory::Mitigation => "mitigation",
        RejectCategory::WeakTest => "weak_test",
        RejectCategory::Regression => "regression",
        RejectCategory::IncompleteImpl => "incomplete",
        RejectCategory::SecurityIssue => "security",
    }
}

/// Build the prompt sent to the verifier.
///
/// Layout:
/// ```text
/// == VERIFIER ROUND {round} ==
///
/// ## Prior rounds
/// - Round 1: ...
/// - Round 2: ...
///
/// ## Planner output
/// <planner_output, possibly truncated>
///
/// Please respond with a JSON verdict: {"verdict": "APPROVED"|"REJECTED", ...}
/// ```
pub fn build_verifier_prompt(
    round: u32,
    planner_output: &str,
    prior_rounds: &[(u32, VerifyResult)],
    max_tokens: usize,
) -> String {
    let mut sections: Vec<String> = Vec::new();

    sections.push(format!("== VERIFIER ROUND {round} =="));

    if !prior_rounds.is_empty() {
        let mut summary = String::from("\n## Prior rounds\n");
        for (r, result) in prior_rounds {
            summary.push_str(&format_round_summary(*r, result));
            summary.push('\n');
        }
        sections.push(summary);
    }

    sections.push(String::from("\n## Planner output\n"));

    // Reserve tokens for the frame (header + prior rounds + footer).
    let frame = sections.join("");
    let frame_tokens = estimate_tokens(&frame);
    // Footer is fixed text.
    let footer = "\nPlease respond with a JSON verdict: {\"verdict\": \"APPROVED\"|\"REJECTED\", \"category\": \"...\", \"reason\": \"...\", \"confidence\": 0.0-1.0}";
    let footer_tokens = estimate_tokens(footer);
    let available = max_tokens.saturating_sub(frame_tokens + footer_tokens);

    let body = truncate_to_budget(planner_output, available);
    sections.push(body);
    sections.push(footer.to_string());

    sections.join("")
}

/// Build the feedback prompt sent back to the planner after a rejection.
///
/// Layout:
/// ```text
/// == PLANNER FEEDBACK (ROUND {round}) ==
///
/// ## Verifier verdict
/// {verdict_summary}
///
/// ## Verifier output
/// <verifier_output, possibly truncated>
///
/// Please revise your plan to address the above feedback.
/// ```
pub fn build_planner_feedback(
    round: u32,
    verifier_output: &str,
    verdict: &VerifyResult,
    max_tokens: usize,
) -> String {
    let mut sections: Vec<String> = Vec::new();

    sections.push(format!("== PLANNER FEEDBACK (ROUND {round}) =="));

    let verdict_summary = match verdict {
        VerifyResult::Approved { reason, confidence } => {
            format!("\n## Verifier verdict\nAPPROVED (confidence={confidence:.2}): {reason}\n")
        }
        VerifyResult::Rejected {
            reason,
            category,
            confidence,
        } => {
            let cat = category_label(category);
            format!(
                "\n## Verifier verdict\nREJECTED [{cat}] (confidence={confidence:.2}): {reason}\n"
            )
        }
        VerifyResult::NeedsInfo { question } => {
            format!("\n## Verifier verdict\nNEEDS_INFO: {question}\n")
        }
    };
    sections.push(verdict_summary);

    sections.push(String::from("## Verifier output\n"));

    let frame = sections.join("");
    let frame_tokens = estimate_tokens(&frame);
    let footer = "\nPlease revise your plan to address the above feedback.";
    let footer_tokens = estimate_tokens(footer);
    let available = max_tokens.saturating_sub(frame_tokens + footer_tokens);

    let body = truncate_to_budget(verifier_output, available);
    sections.push(body);
    sections.push(footer.to_string());

    sections.join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_tokens_ascii_only() {
        // 12 ASCII chars → ceil(12/4) = 3 tokens
        assert_eq!(estimate_tokens("hello world!"), 3);
    }

    #[test]
    fn estimate_tokens_korean_only() {
        // 3 Korean chars → ceil(3/2) = 2 tokens
        assert_eq!(estimate_tokens("안녕하"), 2);
    }

    #[test]
    fn estimate_tokens_mixed() {
        // "hello 안녕" → 6 ASCII (including space) + 2 Korean
        // ceil(6/4) + ceil(2/2) = 2 + 1 = 3
        assert_eq!(estimate_tokens("hello 안녕"), 3);
    }

    #[test]
    fn tail_chars_short() {
        assert_eq!(tail_chars("abc", 10), "abc");
    }

    #[test]
    fn tail_chars_exact() {
        assert_eq!(tail_chars("abcde", 5), "abcde");
    }

    #[test]
    fn truncate_preserves_short_text() {
        let text = "short";
        assert_eq!(truncate_to_budget(text, 100), "short");
    }

    #[test]
    fn truncate_adds_prefix_for_long_text() {
        // Create text that is definitely over 10 tokens.
        let long_text = "a".repeat(200); // 200 ASCII chars = 50 tokens
        let result = truncate_to_budget(&long_text, 10);
        assert!(result.starts_with("[truncated]\n"));
    }
}
