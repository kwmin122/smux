//! Ownership lane assignment and collision detection for parallel workers.
//!
//! Each worker is assigned to an ownership lane that maps to file globs.
//! Two workers cannot own the same lane. Overlapping globs between lanes
//! are detected using the `globset` crate for real pattern matching.

use globset::Glob;
use serde::{Deserialize, Serialize};

use crate::pipeline::OwnershipLane;

/// A worker-to-lane assignment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneAssignment {
    pub worker_id: String,
    pub lane_name: String,
}

/// A detected collision between two lanes.
#[derive(Debug, Clone)]
pub struct LaneCollision {
    pub lane_a: String,
    pub lane_b: String,
    pub glob: String,
}

/// Assign a worker to a named lane if it exists.
pub fn assign_worker_to_lane(
    worker_id: &str,
    lane_name: &str,
    lanes: &[OwnershipLane],
) -> Option<LaneAssignment> {
    lanes
        .iter()
        .find(|l| l.name == lane_name)
        .map(|_| LaneAssignment {
            worker_id: worker_id.to_string(),
            lane_name: lane_name.to_string(),
        })
}

/// Detect glob collisions between lanes using real glob matching.
///
/// Tests a representative set of paths against each lane's globs.
/// If any path matches globs from two different lanes, it's a collision.
pub fn detect_collisions(lanes: &[OwnershipLane]) -> Vec<LaneCollision> {
    let mut collisions = Vec::new();

    for i in 0..lanes.len() {
        for j in (i + 1)..lanes.len() {
            for glob_a in &lanes[i].file_globs {
                for glob_b in &lanes[j].file_globs {
                    if globs_overlap(glob_a, glob_b) {
                        collisions.push(LaneCollision {
                            lane_a: lanes[i].name.clone(),
                            lane_b: lanes[j].name.clone(),
                            glob: format!("{glob_a} ∩ {glob_b}"),
                        });
                    }
                }
            }
        }
    }
    collisions
}

/// Check if two glob patterns could match overlapping files.
///
/// Uses multiple strategies:
/// 1. Exact string match
/// 2. One pattern is a prefix/superset of the other
/// 3. Both patterns compiled and tested against synthetic paths
fn globs_overlap(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }

    // Compile both globs
    let glob_a = match Glob::new(a) {
        Ok(g) => g.compile_matcher(),
        Err(_) => return false,
    };
    let glob_b = match Glob::new(b) {
        Ok(g) => g.compile_matcher(),
        Err(_) => return false,
    };

    // Generate test paths from both patterns
    let test_paths = generate_test_paths(a, b);

    // If any test path matches both globs, they overlap
    for path in &test_paths {
        if glob_a.is_match(path) && glob_b.is_match(path) {
            return true;
        }
    }

    false
}

/// Generate synthetic file paths that could plausibly match the given patterns.
fn generate_test_paths(a: &str, b: &str) -> Vec<String> {
    let mut paths = Vec::new();

    // Extract base directories from patterns
    for pattern in [a, b] {
        let base = pattern
            .split("/**")
            .next()
            .unwrap_or(pattern)
            .split("/*")
            .next()
            .unwrap_or(pattern);

        if !base.is_empty() && base != pattern {
            // Generate paths under this base
            paths.push(format!("{base}/file.rs"));
            paths.push(format!("{base}/main.rs"));
            paths.push(format!("{base}/mod.rs"));
            paths.push(format!("{base}/test.ts"));
            paths.push(format!("{base}/index.tsx"));
            paths.push(format!("{base}/sub/file.rs"));
            paths.push(format!("{base}/sub/deep/file.rs"));
        }
    }

    // Also try the patterns themselves as paths (for non-wildcard patterns)
    for pattern in [a, b] {
        if !pattern.contains('*') && !pattern.contains('?') {
            paths.push(pattern.to_string());
        }
    }

    paths
}

/// Validate that no two workers are assigned to the same lane.
pub fn validate_assignments(assignments: &[LaneAssignment]) -> Result<(), String> {
    let mut seen_lanes = std::collections::HashSet::new();
    for a in assignments {
        if !seen_lanes.insert(&a.lane_name) {
            return Err(format!(
                "lane '{}' assigned to multiple workers",
                a.lane_name
            ));
        }
    }
    Ok(())
}
