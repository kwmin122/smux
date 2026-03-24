//! Ownership lane assignment and collision detection for parallel workers.
//!
//! Each worker is assigned to an ownership lane that maps to file globs.
//! Two workers cannot own the same lane. Overlapping globs between lanes
//! are detected as collisions requiring serialization or user approval.

use serde::{Deserialize, Serialize};

use crate::pipeline::OwnershipLane;

/// A worker-to-lane assignment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneAssignment {
    pub worker_id: String,
    pub lane_name: String,
}

/// A detected collision between two lanes sharing a glob.
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

/// Detect glob collisions between lanes.
/// Checks both exact matches AND containment (e.g., `src/**` contains `src/components/**`).
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

/// Check if two glob patterns overlap.
/// Handles: exact match, prefix containment (src/** contains src/components/**).
fn globs_overlap(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    // Strip trailing /** for prefix comparison
    let prefix_a = a.strip_suffix("/**").unwrap_or(a);
    let prefix_b = b.strip_suffix("/**").unwrap_or(b);

    // If one is a prefix of the other, they overlap
    if a.ends_with("/**") && b.starts_with(prefix_a) {
        return true;
    }
    if b.ends_with("/**") && a.starts_with(prefix_b) {
        return true;
    }
    false
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
