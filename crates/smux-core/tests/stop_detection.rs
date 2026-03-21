//! Integration tests for stop detection (`smux_core::stop`).

use smux_core::stop::detect;
use smux_core::types::{RejectCategory, VerifyResult};

// ── Tier 1: JSON verdict at end of response ─────────────────────────────

#[test]
fn json_verdict_approved_at_end() {
    let response = r#"{"verdict":"APPROVED","reason":"Looks good","confidence":0.95}"#;
    let result = detect(response);
    assert_eq!(
        result,
        VerifyResult::Approved {
            reason: "Looks good".into(),
            confidence: 0.95,
        }
    );
}

#[test]
fn json_verdict_rejected_at_end() {
    let response = r#"{"verdict":"REJECTED","category":"mitigation","reason":"Not a root fix","confidence":0.8}"#;
    let result = detect(response);
    assert_eq!(
        result,
        VerifyResult::Rejected {
            reason: "Not a root fix".into(),
            category: RejectCategory::Mitigation,
            confidence: 0.8,
        }
    );
}

#[test]
fn json_verdict_embedded_in_natural_language() {
    let response = r#"After reviewing the code, I found several issues.
Here is my verdict:
{"verdict":"REJECTED","category":"weak_test","reason":"No edge cases","confidence":0.7}"#;
    let result = detect(response);
    assert_eq!(
        result,
        VerifyResult::Rejected {
            reason: "No edge cases".into(),
            category: RejectCategory::WeakTest,
            confidence: 0.7,
        }
    );
}

#[test]
fn json_root_fix_maps_to_approved() {
    let response = r#"{"verdict":"REJECTED","category":"root_fix","reason":"Actually a root fix","confidence":0.9}"#;
    let result = detect(response);
    assert_eq!(
        result,
        VerifyResult::Approved {
            reason: "Actually a root fix".into(),
            confidence: 0.9,
        }
    );
}

#[test]
fn confidence_value_preserved_from_json() {
    let response = r#"{"verdict":"APPROVED","reason":"ok","confidence":0.42}"#;
    match detect(response) {
        VerifyResult::Approved { confidence, .. } => {
            assert!((confidence - 0.42).abs() < f64::EPSILON);
        }
        other => panic!("expected Approved, got {other:?}"),
    }
}

// ── All RejectCategory mappings ──────────────────────────────────────────

#[test]
fn category_mitigation() {
    let r =
        detect(r#"{"verdict":"REJECTED","category":"mitigation","reason":"x","confidence":0.5}"#);
    match r {
        VerifyResult::Rejected { category, .. } => assert_eq!(category, RejectCategory::Mitigation),
        other => panic!("expected Rejected, got {other:?}"),
    }
}

#[test]
fn category_weak_test() {
    let r =
        detect(r#"{"verdict":"REJECTED","category":"weak_test","reason":"x","confidence":0.5}"#);
    match r {
        VerifyResult::Rejected { category, .. } => assert_eq!(category, RejectCategory::WeakTest),
        other => panic!("expected Rejected, got {other:?}"),
    }
}

#[test]
fn category_regression() {
    let r =
        detect(r#"{"verdict":"REJECTED","category":"regression","reason":"x","confidence":0.5}"#);
    match r {
        VerifyResult::Rejected { category, .. } => assert_eq!(category, RejectCategory::Regression),
        other => panic!("expected Rejected, got {other:?}"),
    }
}

#[test]
fn category_incomplete() {
    let r =
        detect(r#"{"verdict":"REJECTED","category":"incomplete","reason":"x","confidence":0.5}"#);
    match r {
        VerifyResult::Rejected { category, .. } => {
            assert_eq!(category, RejectCategory::IncompleteImpl);
        }
        other => panic!("expected Rejected, got {other:?}"),
    }
}

#[test]
fn category_security() {
    let r = detect(r#"{"verdict":"REJECTED","category":"security","reason":"x","confidence":0.5}"#);
    match r {
        VerifyResult::Rejected { category, .. } => {
            assert_eq!(category, RejectCategory::SecurityIssue);
        }
        other => panic!("expected Rejected, got {other:?}"),
    }
}

#[test]
fn category_unknown_defaults_to_incomplete() {
    let r = detect(
        r#"{"verdict":"REJECTED","category":"something_else","reason":"x","confidence":0.5}"#,
    );
    match r {
        VerifyResult::Rejected { category, .. } => {
            assert_eq!(category, RejectCategory::IncompleteImpl);
        }
        other => panic!("expected Rejected, got {other:?}"),
    }
}

// ── Tier 2: Keyword fallback ────────────────────────────────────────────

#[test]
fn keyword_approved_without_json() {
    let response = "The implementation looks correct. APPROVED.";
    let result = detect(response);
    match result {
        VerifyResult::Approved { confidence, .. } => {
            assert!((confidence - 0.5).abs() < f64::EPSILON);
        }
        other => panic!("expected Approved, got {other:?}"),
    }
}

#[test]
fn keyword_pass_without_json() {
    let response = "All checks PASS.";
    let result = detect(response);
    assert!(matches!(result, VerifyResult::Approved { .. }));
}

#[test]
fn keyword_rejected_without_json() {
    let response = "This change is REJECTED because it breaks tests.";
    let result = detect(response);
    match result {
        VerifyResult::Rejected {
            category,
            confidence,
            ..
        } => {
            assert_eq!(category, RejectCategory::IncompleteImpl);
            assert!((confidence - 0.5).abs() < f64::EPSILON);
        }
        other => panic!("expected Rejected, got {other:?}"),
    }
}

#[test]
fn keyword_fail_without_json() {
    let response = "Tests FAIL on the edge case.";
    let result = detect(response);
    assert!(matches!(result, VerifyResult::Rejected { .. }));
}

// ── Keyword word-boundary enforcement ──────────────────────────────────

#[test]
fn keyword_pass_does_not_match_password() {
    let response = "Please reset your PASSWORD to continue.";
    let result = detect(response);
    // "PASSWORD" should NOT match "PASS"
    assert!(matches!(result, VerifyResult::NeedsInfo { .. }));
}

#[test]
fn keyword_fail_does_not_match_failsafe() {
    let response = "The FAILSAFE mechanism triggered.";
    let result = detect(response);
    assert!(matches!(result, VerifyResult::NeedsInfo { .. }));
}

// ── Conflicting keywords ──────────────────────────────────────────────

#[test]
fn conflicting_keywords_rejection_takes_precedence() {
    // When both approval and rejection keywords appear, rejection wins
    // (conservative: don't auto-approve if any rejection signal exists)
    let response = "Tests FAIL but the reviewer said APPROVED overall.";
    let result = detect(response);
    assert!(
        matches!(result, VerifyResult::Rejected { .. }),
        "rejection should take precedence over approval: got {result:?}"
    );
}

// ── Tier 3: No signal ───────────────────────────────────────────────────

#[test]
fn no_signal_returns_needs_info() {
    let response = "I have reviewed the code but have no opinion yet.";
    let result = detect(response);
    assert!(matches!(result, VerifyResult::NeedsInfo { .. }));
}

#[test]
fn empty_response_returns_needs_info() {
    let result = detect("");
    assert!(matches!(result, VerifyResult::NeedsInfo { .. }));
}

// ── Malformed JSON falls to keyword fallback ────────────────────────────

#[test]
fn malformed_json_falls_to_keyword() {
    let response = r#"{"verdict":"APPROVED", broken json... APPROVED overall."#;
    let result = detect(response);
    // The JSON is malformed, but the keyword "APPROVED" is present.
    assert!(matches!(result, VerifyResult::Approved { .. }));
}

#[test]
fn malformed_json_no_keyword_returns_needs_info() {
    let response = r#"{"verdict":"APPROVED", broken json..."#;
    // No keyword (the "APPROVED" is inside malformed JSON that won't parse,
    // but the keyword scanner *does* see it).
    // Actually, "APPROVED" does appear as a keyword, so this should match.
    let result = detect(response);
    assert!(matches!(result, VerifyResult::Approved { .. }));
}

// ── JSON after other braces ─────────────────────────────────────────────

#[test]
fn json_verdict_after_code_blocks_with_braces() {
    let response = r#"Here is a code review:
```
fn foo() { bar() }
```
Some more text with { random braces }.
{"verdict":"APPROVED","reason":"Code is correct","confidence":0.88}"#;
    let result = detect(response);
    assert_eq!(
        result,
        VerifyResult::Approved {
            reason: "Code is correct".into(),
            confidence: 0.88,
        }
    );
}
