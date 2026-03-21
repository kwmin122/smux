//! Stop detection — parse verifier output into a [`VerifyResult`].
//!
//! Detection chain (3 tiers):
//! 1. **JSON verdict block** — find the last `{"verdict":` object in the response.
//! 2. **Keyword fallback** — scan for `APPROVED`/`REJECTED`/`PASS`/`FAIL`.
//! 3. **No signal** — return `NeedsInfo`.

use serde_json::Value;

use crate::types::{RejectCategory, VerifyResult};

/// Default confidence when falling back to keyword detection.
const DEFAULT_KEYWORD_CONFIDENCE: f64 = 0.5;

/// Parse verifier output into a [`VerifyResult`].
///
/// Searches the response for a JSON verdict block first, then falls back to
/// keyword scanning, and finally returns `NeedsInfo` if nothing is found.
pub fn detect(response: &str) -> VerifyResult {
    if let Some(result) = try_json_verdict(response) {
        return result;
    }
    if let Some(result) = try_keyword_fallback(response) {
        return result;
    }
    VerifyResult::NeedsInfo {
        question: "Please provide a verdict (APPROVED or REJECTED) with a reason.".into(),
    }
}

// ── Tier 1: JSON verdict ────────────────────────────────────────────────

/// Search backwards from the end of `response` for the last `{` that starts a
/// valid verdict JSON object.  Returns `None` if no valid verdict is found.
fn try_json_verdict(response: &str) -> Option<VerifyResult> {
    // Walk backwards through all `{` positions.
    let bytes = response.as_bytes();
    let mut search_from = bytes.len();
    while search_from > 0 {
        let pos = response[..search_from].rfind('{')?;
        let candidate = &response[pos..];
        if let Some(result) = parse_verdict_json(candidate) {
            return Some(result);
        }
        search_from = pos;
    }
    None
}

/// Try to parse `text` (starting from `{`) as a verdict JSON object.
fn parse_verdict_json(text: &str) -> Option<VerifyResult> {
    // Find the matching closing brace — take the first complete object.
    let end = find_matching_brace(text)?;
    let json_str = &text[..=end];
    let val: Value = serde_json::from_str(json_str).ok()?;
    let obj = val.as_object()?;

    let verdict = obj.get("verdict")?.as_str()?.to_uppercase();
    let reason = obj
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let confidence = obj
        .get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let category_str = obj
        .get("category")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();

    match verdict.as_str() {
        "APPROVED" => Some(VerifyResult::Approved { reason, confidence }),
        "REJECTED" => {
            // "root_fix" is not a rejection — treat as Approved.
            if category_str == "root_fix" {
                return Some(VerifyResult::Approved { reason, confidence });
            }
            let category = map_category(&category_str);
            Some(VerifyResult::Rejected {
                reason,
                category,
                confidence,
            })
        }
        _ => None,
    }
}

/// Find the index of the closing `}` that matches the opening `{` at index 0.
fn find_matching_brace(text: &str) -> Option<usize> {
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escape = false;
    for (i, ch) in text.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' && in_string {
            escape = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Map a category string from the verdict JSON to a [`RejectCategory`].
fn map_category(s: &str) -> RejectCategory {
    match s {
        "mitigation" => RejectCategory::Mitigation,
        "weak_test" => RejectCategory::WeakTest,
        "regression" => RejectCategory::Regression,
        "incomplete" => RejectCategory::IncompleteImpl,
        "security" => RejectCategory::SecurityIssue,
        _ => RejectCategory::IncompleteImpl,
    }
}

// ── Tier 2: Keyword fallback ────────────────────────────────────────────

/// Scan for approval/rejection keywords and return a `VerifyResult` with
/// default confidence.
fn try_keyword_fallback(response: &str) -> Option<VerifyResult> {
    let upper = response.to_uppercase();
    if upper.contains("APPROVED") || upper.contains("PASS") {
        return Some(VerifyResult::Approved {
            reason: "Detected approval keyword in response".into(),
            confidence: DEFAULT_KEYWORD_CONFIDENCE,
        });
    }
    if upper.contains("REJECTED") || upper.contains("FAIL") {
        return Some(VerifyResult::Rejected {
            reason: "Detected rejection keyword in response".into(),
            category: RejectCategory::IncompleteImpl,
            confidence: DEFAULT_KEYWORD_CONFIDENCE,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_matching_brace_simple() {
        assert_eq!(find_matching_brace(r#"{"a":1}"#), Some(6));
    }

    #[test]
    fn find_matching_brace_nested() {
        assert_eq!(find_matching_brace(r#"{"a":{"b":2}}"#), Some(12));
    }

    #[test]
    fn find_matching_brace_with_string_braces() {
        assert_eq!(find_matching_brace(r#"{"a":"}"}"#), Some(8));
    }

    #[test]
    fn map_all_categories() {
        assert_eq!(map_category("mitigation"), RejectCategory::Mitigation);
        assert_eq!(map_category("weak_test"), RejectCategory::WeakTest);
        assert_eq!(map_category("regression"), RejectCategory::Regression);
        assert_eq!(map_category("incomplete"), RejectCategory::IncompleteImpl);
        assert_eq!(map_category("security"), RejectCategory::SecurityIssue);
        assert_eq!(map_category("unknown"), RejectCategory::IncompleteImpl);
    }
}
