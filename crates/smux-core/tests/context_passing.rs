//! Integration tests for context passing (`smux_core::context`).

use smux_core::context::{build_planner_feedback, build_verifier_prompt, estimate_tokens};
use smux_core::types::{RejectCategory, VerifyResult};

// ── Token estimation ────────────────────────────────────────────────────

#[test]
fn estimate_tokens_ascii() {
    // "abcd" = 4 ASCII chars → ceil(4/4) = 1 token
    assert_eq!(estimate_tokens("abcd"), 1);
    // "abcde" = 5 ASCII chars → ceil(5/4) = 2 tokens
    assert_eq!(estimate_tokens("abcde"), 2);
}

#[test]
fn estimate_tokens_korean() {
    // "안녕" = 2 Korean chars → ceil(2/2) = 1 token
    assert_eq!(estimate_tokens("안녕"), 1);
    // "안녕하" = 3 Korean chars → ceil(3/2) = 2 tokens
    assert_eq!(estimate_tokens("안녕하"), 2);
}

#[test]
fn estimate_tokens_mixed() {
    // "hi 안녕" → 3 ASCII (h,i,space) + 2 Korean
    // ceil(3/4) + ceil(2/2) = 1 + 1 = 2
    assert_eq!(estimate_tokens("hi 안녕"), 2);
}

#[test]
fn estimate_tokens_empty() {
    assert_eq!(estimate_tokens(""), 0);
}

// ── build_verifier_prompt: short text stays under threshold ─────────────

#[test]
fn short_planner_output_passed_through() {
    let prompt = build_verifier_prompt(1, "Everything looks good.", &[], 4000);
    assert!(prompt.contains("Everything looks good."));
    assert!(!prompt.contains("[truncated]"));
    assert!(prompt.contains("[smux → Verifier, Round 1]"));
}

// ── build_verifier_prompt: long text truncated ──────────────────────────

#[test]
fn long_planner_output_truncated() {
    // Create a very long planner output (well over 4000 tokens).
    let long_output = "a".repeat(40_000); // 40k ASCII chars = 10_000 tokens
    let prompt = build_verifier_prompt(3, &long_output, &[], 4000);
    assert!(prompt.contains("[truncated"));
    // The full 40k should NOT be present.
    assert!(prompt.len() < 40_000);
}

// ── build_verifier_prompt: prior round summaries ────────────────────────

#[test]
fn prior_rounds_included_as_bullets() {
    let prior = vec![
        (
            1,
            VerifyResult::Rejected {
                reason: "Missing tests".into(),
                category: RejectCategory::WeakTest,
                confidence: 0.7,
            },
        ),
        (
            2,
            VerifyResult::Approved {
                reason: "Fixed".into(),
                confidence: 0.9,
            },
        ),
    ];
    let prompt = build_verifier_prompt(3, "plan v3", &prior, 4000);
    assert!(prompt.contains("Previous Rounds Summary"));
    assert!(prompt.contains("R1: REJECTED (weak_test)"));
    assert!(prompt.contains("R2: APPROVED"));
    assert!(prompt.contains("Missing tests"));
}

#[test]
fn empty_prior_rounds_no_summary_section() {
    let prompt = build_verifier_prompt(1, "first plan", &[], 4000);
    assert!(!prompt.contains("Previous Rounds Summary"));
}

// ── build_planner_feedback ──────────────────────────────────────────────

#[test]
fn planner_feedback_includes_verdict_category() {
    let verdict = VerifyResult::Rejected {
        reason: "Not a root fix".into(),
        category: RejectCategory::Mitigation,
        confidence: 0.85,
    };
    let feedback = build_planner_feedback(
        2,
        "The change is a workaround, not a root fix.",
        &verdict,
        4000,
    );
    assert!(feedback.contains("PLANNER FEEDBACK (ROUND 2)"));
    assert!(feedback.contains("REJECTED [mitigation]"));
    assert!(feedback.contains("Not a root fix"));
    assert!(feedback.contains("The change is a workaround"));
    assert!(feedback.contains("Please revise"));
}

#[test]
fn planner_feedback_approved() {
    let verdict = VerifyResult::Approved {
        reason: "All good".into(),
        confidence: 0.95,
    };
    let feedback = build_planner_feedback(1, "Verifier output text", &verdict, 4000);
    assert!(feedback.contains("APPROVED"));
    assert!(feedback.contains("All good"));
}

#[test]
fn planner_feedback_needs_info() {
    let verdict = VerifyResult::NeedsInfo {
        question: "What about error handling?".into(),
    };
    let feedback = build_planner_feedback(1, "output", &verdict, 4000);
    assert!(feedback.contains("NEEDS_INFO"));
    assert!(feedback.contains("What about error handling?"));
}

#[test]
fn planner_feedback_long_output_truncated() {
    let long_output = "b".repeat(40_000);
    let verdict = VerifyResult::Approved {
        reason: "ok".into(),
        confidence: 0.9,
    };
    let feedback = build_planner_feedback(1, &long_output, &verdict, 4000);
    assert!(feedback.contains("[truncated"));
    assert!(feedback.len() < 40_000);
}

// ── Korean text token estimation in context ─────────────────────────────

#[test]
fn korean_text_estimation_in_verifier_prompt() {
    // Korean text with known token count.
    let korean = "안녕하세요".repeat(100); // 500 Korean chars
    // 500 Korean chars → ceil(500/2) = 250 tokens, well under 4000.
    let prompt = build_verifier_prompt(1, &korean, &[], 4000);
    assert!(!prompt.contains("[truncated]"));
    assert!(prompt.contains(&korean));
}

#[test]
fn korean_text_truncated_when_over_budget() {
    // Make Korean text that exceeds the budget.
    let korean = "한".repeat(20_000); // 20k Korean chars → 10_000 tokens
    let prompt = build_verifier_prompt(1, &korean, &[], 4000);
    assert!(prompt.contains("[truncated"));
    assert!(prompt.len() < 60_000); // 20k chars * 3 bytes each = 60k bytes
}
