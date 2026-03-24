//! Tests for ownership lane assignment and collision detection.

use smux_core::ownership::{
    LaneAssignment, assign_worker_to_lane, detect_collisions, validate_assignments,
};
use smux_core::pipeline::OwnershipLane;

#[test]
fn assign_worker_to_matching_lane() {
    let lanes = vec![
        OwnershipLane {
            name: "frontend".into(),
            file_globs: vec!["src/components/**".into()],
        },
        OwnershipLane {
            name: "backend".into(),
            file_globs: vec!["crates/**".into()],
        },
    ];
    let assignment = assign_worker_to_lane("frontend-worker", "frontend", &lanes);
    assert!(assignment.is_some());
    assert_eq!(assignment.unwrap().lane_name, "frontend");
}

#[test]
fn assign_worker_to_nonexistent_lane_fails() {
    let lanes = vec![OwnershipLane {
        name: "frontend".into(),
        file_globs: vec!["src/**".into()],
    }];
    let assignment = assign_worker_to_lane("worker1", "database", &lanes);
    assert!(assignment.is_none());
}

#[test]
fn detect_glob_collision_between_lanes() {
    let lanes = vec![
        OwnershipLane {
            name: "frontend".into(),
            file_globs: vec!["src/**".into()],
        },
        OwnershipLane {
            name: "fullstack".into(),
            file_globs: vec!["src/**".into()],
        },
    ];
    let collisions = detect_collisions(&lanes);
    assert_eq!(collisions.len(), 1);
    assert!(collisions[0].glob.contains("src/**"));
}

#[test]
fn no_collision_when_globs_differ() {
    let lanes = vec![
        OwnershipLane {
            name: "frontend".into(),
            file_globs: vec!["src/components/**".into()],
        },
        OwnershipLane {
            name: "backend".into(),
            file_globs: vec!["crates/**".into()],
        },
    ];
    let collisions = detect_collisions(&lanes);
    assert!(collisions.is_empty());
}

#[test]
fn validate_no_duplicate_worker_assignments() {
    let assignments = vec![
        LaneAssignment {
            worker_id: "w1".into(),
            lane_name: "frontend".into(),
        },
        LaneAssignment {
            worker_id: "w2".into(),
            lane_name: "backend".into(),
        },
    ];
    assert!(validate_assignments(&assignments).is_ok());
}

#[test]
fn validate_duplicate_lane_assignment_rejected() {
    let assignments = vec![
        LaneAssignment {
            worker_id: "w1".into(),
            lane_name: "frontend".into(),
        },
        LaneAssignment {
            worker_id: "w2".into(),
            lane_name: "frontend".into(),
        },
    ];
    assert!(validate_assignments(&assignments).is_err());
}

#[test]
fn empty_lanes_no_collisions() {
    let lanes: Vec<OwnershipLane> = vec![];
    assert!(detect_collisions(&lanes).is_empty());
}

#[test]
fn detect_containment_collision() {
    // src/** contains src/components/** — this must be detected
    let lanes = vec![
        OwnershipLane {
            name: "fullstack".into(),
            file_globs: vec!["src/**".into()],
        },
        OwnershipLane {
            name: "frontend".into(),
            file_globs: vec!["src/components/**".into()],
        },
    ];
    let collisions = detect_collisions(&lanes);
    assert!(
        !collisions.is_empty(),
        "src/** should overlap with src/components/**"
    );
}

#[test]
fn multiple_globs_per_lane() {
    let lanes = vec![OwnershipLane {
        name: "frontend".into(),
        file_globs: vec![
            "src/components/**".into(),
            "src/hooks/**".into(),
            "src/App.tsx".into(),
        ],
    }];
    let assignment = assign_worker_to_lane("fe-worker", "frontend", &lanes);
    assert!(assignment.is_some());
    assert_eq!(assignment.unwrap().lane_name, "frontend");
}
