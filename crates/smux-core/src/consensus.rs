//! Consensus engine for combining multiple verifier verdicts.
//!
//! Pure logic, no IO dependencies. Implements four strategies:
//! Majority (default), Weighted, Unanimous, and LeaderDelegate.

use crate::types::{
    ConsensusResult, ConsensusStrategy, RejectCategory, VerifierVerdict, VerifyResult,
};

/// Resolve multiple verifier verdicts into a single consensus result.
pub fn resolve(strategy: &ConsensusStrategy, verdicts: Vec<VerifierVerdict>) -> ConsensusResult {
    if verdicts.is_empty() {
        return ConsensusResult {
            individual: vec![],
            final_verdict: VerifyResult::Rejected {
                reason: "no verifiers provided".into(),
                category: RejectCategory::IncompleteImpl,
                confidence: 0.0,
            },
            strategy: strategy.clone(),
            agreement_ratio: 0.0,
        };
    }

    let final_verdict = match strategy {
        ConsensusStrategy::Majority => majority(&verdicts),
        ConsensusStrategy::Weighted => weighted(&verdicts),
        ConsensusStrategy::Unanimous => unanimous(&verdicts),
        ConsensusStrategy::LeaderDelegate => leader_delegate(&verdicts),
    };

    let agreement_ratio = compute_agreement(&verdicts, &final_verdict);

    ConsensusResult {
        individual: verdicts,
        final_verdict,
        strategy: strategy.clone(),
        agreement_ratio,
    }
}

/// Majority: >50% approved → approved. Otherwise rejected.
fn majority(verdicts: &[VerifierVerdict]) -> VerifyResult {
    let total = verdicts.len();
    let approved_count = verdicts
        .iter()
        .filter(|v| matches!(v.result, VerifyResult::Approved { .. }))
        .count();

    if approved_count * 2 > total {
        // Approved — use the reason from the highest-confidence approval.
        let best = verdicts
            .iter()
            .filter_map(|v| match &v.result {
                VerifyResult::Approved { reason, confidence } => Some((reason, *confidence)),
                _ => None,
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        match best {
            Some((reason, confidence)) => VerifyResult::Approved {
                reason: reason.clone(),
                confidence,
            },
            None => VerifyResult::Approved {
                reason: "majority approved".into(),
                confidence: 0.5,
            },
        }
    } else {
        // Rejected — use the reason from the highest-confidence rejection.
        best_rejection(verdicts)
    }
}

/// Weighted: confidence-weighted average. If average confidence of approvals > 0.5 → approved.
fn weighted(verdicts: &[VerifierVerdict]) -> VerifyResult {
    let mut weighted_approve = 0.0_f64;
    let mut weighted_reject = 0.0_f64;

    for v in verdicts {
        match &v.result {
            VerifyResult::Approved { confidence, .. } => weighted_approve += confidence,
            VerifyResult::Rejected { confidence, .. } => weighted_reject += confidence,
            VerifyResult::NeedsInfo { .. } => {} // NeedsInfo contributes 0
        }
    }

    let total_weight = weighted_approve + weighted_reject;
    if total_weight == 0.0 {
        // All NeedsInfo — treat as rejection.
        return VerifyResult::Rejected {
            reason: "all verifiers returned NeedsInfo".into(),
            category: RejectCategory::IncompleteImpl,
            confidence: 0.0,
        };
    }

    let approval_ratio = weighted_approve / total_weight;

    if approval_ratio > 0.5 {
        let best = verdicts
            .iter()
            .filter_map(|v| match &v.result {
                VerifyResult::Approved { reason, confidence } => Some((reason, *confidence)),
                _ => None,
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        match best {
            Some((reason, _)) => VerifyResult::Approved {
                reason: reason.clone(),
                confidence: approval_ratio,
            },
            None => VerifyResult::Approved {
                reason: "weighted approval".into(),
                confidence: approval_ratio,
            },
        }
    } else {
        best_rejection(verdicts)
    }
}

/// Unanimous: all must approve. Any rejection → rejected.
fn unanimous(verdicts: &[VerifierVerdict]) -> VerifyResult {
    let all_approved = verdicts
        .iter()
        .all(|v| matches!(v.result, VerifyResult::Approved { .. }));

    if all_approved {
        // Average confidence.
        let avg_conf: f64 = verdicts
            .iter()
            .filter_map(|v| match &v.result {
                VerifyResult::Approved { confidence, .. } => Some(*confidence),
                _ => None,
            })
            .sum::<f64>()
            / verdicts.len() as f64;

        VerifyResult::Approved {
            reason: "unanimous approval".into(),
            confidence: avg_conf,
        }
    } else {
        best_rejection(verdicts)
    }
}

/// LeaderDelegate: the first verifier (leader) decides, others are advisory.
fn leader_delegate(verdicts: &[VerifierVerdict]) -> VerifyResult {
    verdicts[0].result.clone()
}

/// Extract the best (highest-confidence) rejection reason, falling back to a default.
fn best_rejection(verdicts: &[VerifierVerdict]) -> VerifyResult {
    let best = verdicts
        .iter()
        .filter_map(|v| match &v.result {
            VerifyResult::Rejected {
                reason,
                category,
                confidence,
            } => Some((reason, category, *confidence)),
            _ => None,
        })
        .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

    match best {
        Some((reason, category, confidence)) => VerifyResult::Rejected {
            reason: reason.clone(),
            category: category.clone(),
            confidence,
        },
        None => VerifyResult::Rejected {
            reason: "no clear verdict".into(),
            category: RejectCategory::IncompleteImpl,
            confidence: 0.0,
        },
    }
}

/// Compute the fraction of verdicts that match the final verdict direction.
fn compute_agreement(verdicts: &[VerifierVerdict], final_verdict: &VerifyResult) -> f64 {
    let is_approved = matches!(final_verdict, VerifyResult::Approved { .. });
    let matching = verdicts
        .iter()
        .filter(|v| matches!(v.result, VerifyResult::Approved { .. }) == is_approved)
        .count();
    matching as f64 / verdicts.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approved(verifier: &str, confidence: f64) -> VerifierVerdict {
        VerifierVerdict {
            adapter_name: verifier.into(),
            result: VerifyResult::Approved {
                reason: format!("{verifier} approved"),
                confidence,
            },
            duration_ms: 0,
        }
    }

    fn rejected(verifier: &str, confidence: f64) -> VerifierVerdict {
        VerifierVerdict {
            adapter_name: verifier.into(),
            result: VerifyResult::Rejected {
                reason: format!("{verifier} rejected"),
                category: RejectCategory::IncompleteImpl,
                confidence,
            },
            duration_ms: 0,
        }
    }

    fn needs_info(verifier: &str) -> VerifierVerdict {
        VerifierVerdict {
            adapter_name: verifier.into(),
            result: VerifyResult::NeedsInfo {
                question: "what?".into(),
            },
            duration_ms: 0,
        }
    }

    // ── Majority ──

    #[test]
    fn majority_2_of_3_approved() {
        let result = resolve(
            &ConsensusStrategy::Majority,
            vec![approved("a", 0.9), approved("b", 0.8), rejected("c", 0.7)],
        );
        assert!(matches!(
            result.final_verdict,
            VerifyResult::Approved { .. }
        ));
        assert!((result.agreement_ratio - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn majority_1_of_3_rejected() {
        let result = resolve(
            &ConsensusStrategy::Majority,
            vec![approved("a", 0.5), rejected("b", 0.9), rejected("c", 0.8)],
        );
        assert!(matches!(
            result.final_verdict,
            VerifyResult::Rejected { .. }
        ));
        assert!((result.agreement_ratio - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn majority_single_verifier() {
        let result = resolve(&ConsensusStrategy::Majority, vec![approved("a", 0.95)]);
        assert!(matches!(
            result.final_verdict,
            VerifyResult::Approved { .. }
        ));
        assert!((result.agreement_ratio - 1.0).abs() < 0.01);
    }

    // ── Weighted ──

    #[test]
    fn weighted_high_confidence_approval_wins() {
        let result = resolve(
            &ConsensusStrategy::Weighted,
            vec![approved("a", 0.9), rejected("b", 0.3), rejected("c", 0.2)],
        );
        // weighted_approve = 0.9, weighted_reject = 0.5, ratio = 0.9/1.4 ≈ 0.64 > 0.5
        assert!(matches!(
            result.final_verdict,
            VerifyResult::Approved { .. }
        ));
    }

    #[test]
    fn weighted_high_confidence_rejection_wins() {
        let result = resolve(
            &ConsensusStrategy::Weighted,
            vec![approved("a", 0.3), rejected("b", 0.9), rejected("c", 0.8)],
        );
        // weighted_approve = 0.3, weighted_reject = 1.7, ratio = 0.3/2.0 = 0.15 < 0.5
        assert!(matches!(
            result.final_verdict,
            VerifyResult::Rejected { .. }
        ));
    }

    #[test]
    fn weighted_all_needs_info() {
        let result = resolve(
            &ConsensusStrategy::Weighted,
            vec![needs_info("a"), needs_info("b")],
        );
        assert!(matches!(
            result.final_verdict,
            VerifyResult::Rejected { .. }
        ));
    }

    // ── Unanimous ──

    #[test]
    fn unanimous_all_approved() {
        let result = resolve(
            &ConsensusStrategy::Unanimous,
            vec![approved("a", 0.9), approved("b", 0.8), approved("c", 0.7)],
        );
        assert!(matches!(
            result.final_verdict,
            VerifyResult::Approved { .. }
        ));
        assert!((result.agreement_ratio - 1.0).abs() < 0.01);
    }

    #[test]
    fn unanimous_one_rejection_blocks() {
        let result = resolve(
            &ConsensusStrategy::Unanimous,
            vec![approved("a", 0.9), approved("b", 0.8), rejected("c", 0.7)],
        );
        assert!(matches!(
            result.final_verdict,
            VerifyResult::Rejected { .. }
        ));
    }

    // ── LeaderDelegate ──

    #[test]
    fn leader_delegate_leader_decides() {
        let result = resolve(
            &ConsensusStrategy::LeaderDelegate,
            vec![
                rejected("leader", 0.95),
                approved("b", 0.9),
                approved("c", 0.8),
            ],
        );
        assert!(matches!(
            result.final_verdict,
            VerifyResult::Rejected { .. }
        ));
    }

    #[test]
    fn leader_delegate_leader_approves() {
        let result = resolve(
            &ConsensusStrategy::LeaderDelegate,
            vec![approved("leader", 0.85), rejected("b", 0.9)],
        );
        assert!(matches!(
            result.final_verdict,
            VerifyResult::Approved { .. }
        ));
    }

    // ── Agreement ratio ──

    #[test]
    fn agreement_ratio_unanimous() {
        let result = resolve(
            &ConsensusStrategy::Majority,
            vec![approved("a", 0.9), approved("b", 0.8)],
        );
        assert!((result.agreement_ratio - 1.0).abs() < 0.01);
    }

    #[test]
    fn agreement_ratio_split() {
        let result = resolve(
            &ConsensusStrategy::Majority,
            vec![approved("a", 0.9), approved("b", 0.8), rejected("c", 0.7)],
        );
        // 2 of 3 agree with final (approved) → 0.667
        assert!((result.agreement_ratio - 2.0 / 3.0).abs() < 0.01);
    }
}
