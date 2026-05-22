use chrono::NaiveDate;
use thiserror::Error;
use std::collections::HashSet;

use crate::schema::{Patch, PatchOp, TaskRef, PATCH_VERSION};

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("unsupported patch_version: got {0}, this binary supports {}", PATCH_VERSION)]
    UnsupportedVersion(u32),

    #[error("patch.ops must not be empty")]
    EmptyOps,

    #[error("patch.summary must not be empty")]
    EmptySummary,

    #[error("op #{op_index}: bad date {value:?} (expected YYYY-MM-DD)")]
    BadDate { op_index: usize, value: String },

    #[error("op #{op_index}: duration_workdays must be >= 1, got {got}")]
    NonPositiveDuration { op_index: usize, got: i64 },

    #[error("op #{op_index}: unknown dep_type {value:?} (expected FS/SS/FF/SF)")]
    BadDepType { op_index: usize, value: String },

    #[error("op #{op_index}: op_ref {value:?} does not match any add_task in this patch")]
    DanglingOpRef { op_index: usize, value: String },

    #[error("op_ref {value:?} declared more than once in the same patch")]
    DuplicateOpRef { value: String },

    #[error("op #{op_index}: empty name")]
    EmptyName { op_index: usize },

    #[error("op #{op_index}: empty text")]
    EmptyText { op_index: usize },
}

const VALID_DEP_TYPES: &[&str] = &["FS", "SS", "FF", "SF"];

/// Validates a patch's *structural* soundness — shape, value ranges,
/// op_ref consistency. Referential integrity against the database
/// (does task 42 exist?) is checked at apply-time in Plan 3, not here,
/// because the MCP server may run against a snapshot the user has
/// since edited.
pub fn validate_patch(p: &Patch) -> Result<(), ValidationError> {
    if p.patch_version != PATCH_VERSION {
        return Err(ValidationError::UnsupportedVersion(p.patch_version));
    }
    if p.summary.trim().is_empty() {
        return Err(ValidationError::EmptySummary);
    }
    if p.ops.is_empty() {
        return Err(ValidationError::EmptyOps);
    }

    // First pass: collect all declared op_refs, checking for duplicates.
    let mut declared: HashSet<String> = HashSet::new();
    for op in &p.ops {
        if let PatchOp::AddTask { op_ref: Some(r), .. } = op {
            if !declared.insert(r.clone()) {
                return Err(ValidationError::DuplicateOpRef { value: r.clone() });
            }
        }
    }

    // Second pass: per-op structural checks.
    for (i, op) in p.ops.iter().enumerate() {
        match op {
            PatchOp::AddTask {
                name, start_date, duration_workdays, ..
            } => {
                if name.trim().is_empty() {
                    return Err(ValidationError::EmptyName { op_index: i });
                }
                parse_date(i, start_date)?;
                if *duration_workdays < 1 {
                    return Err(ValidationError::NonPositiveDuration {
                        op_index: i,
                        got: *duration_workdays,
                    });
                }
            }
            PatchOp::ShiftTask { .. } => { /* nothing structural to check */ }
            PatchOp::AddDependency {
                predecessor, successor, dep_type, ..
            } => {
                if !VALID_DEP_TYPES.contains(&dep_type.as_str()) {
                    return Err(ValidationError::BadDepType {
                        op_index: i,
                        value: dep_type.clone(),
                    });
                }
                check_taskref(i, predecessor, &declared)?;
                check_taskref(i, successor, &declared)?;
            }
            PatchOp::AddChaser { .. } => { /* template-name check happens at apply-time */ }
            PatchOp::AppendNote { text, .. } => {
                if text.trim().is_empty() {
                    return Err(ValidationError::EmptyText { op_index: i });
                }
            }
        }
    }

    Ok(())
}

fn parse_date(op_index: usize, s: &str) -> Result<NaiveDate, ValidationError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| ValidationError::BadDate {
        op_index,
        value: s.to_string(),
    })
}

fn check_taskref(
    op_index: usize,
    r: &TaskRef,
    declared: &HashSet<String>,
) -> Result<(), ValidationError> {
    match r {
        TaskRef::Existing { .. } => Ok(()),
        TaskRef::Pending { op_ref } => {
            if declared.contains(op_ref) {
                Ok(())
            } else {
                Err(ValidationError::DanglingOpRef {
                    op_index,
                    value: op_ref.clone(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Patch, PatchOp, TaskRef};

    fn ok_patch() -> Patch {
        Patch {
            patch_version: 1,
            summary: "x".into(),
            ops: vec![PatchOp::AppendNote {
                job_id: 1,
                text: "hi".into(),
            }],
        }
    }

    #[test]
    fn accepts_well_formed_patch() {
        assert!(validate_patch(&ok_patch()).is_ok());
    }

    #[test]
    fn rejects_unknown_patch_version() {
        let mut p = ok_patch();
        p.patch_version = 99;
        let err = validate_patch(&p).unwrap_err();
        assert!(matches!(err, ValidationError::UnsupportedVersion(99)));
    }

    #[test]
    fn rejects_empty_ops() {
        let mut p = ok_patch();
        p.ops.clear();
        assert!(matches!(validate_patch(&p).unwrap_err(), ValidationError::EmptyOps));
    }

    #[test]
    fn rejects_empty_summary() {
        let mut p = ok_patch();
        p.summary = "".into();
        assert!(matches!(
            validate_patch(&p).unwrap_err(),
            ValidationError::EmptySummary
        ));
    }

    #[test]
    fn rejects_bad_date_in_add_task() {
        let p = Patch {
            patch_version: 1,
            summary: "x".into(),
            ops: vec![PatchOp::AddTask {
                phase_id: 1,
                name: "n".into(),
                start_date: "not-a-date".into(),
                duration_workdays: 1,
                notes: None,
                contact_id: None,
                op_ref: None,
            }],
        };
        let err = validate_patch(&p).unwrap_err();
        assert!(matches!(err, ValidationError::BadDate { .. }));
    }

    #[test]
    fn rejects_non_positive_duration() {
        let p = Patch {
            patch_version: 1,
            summary: "x".into(),
            ops: vec![PatchOp::AddTask {
                phase_id: 1,
                name: "n".into(),
                start_date: "2026-06-03".into(),
                duration_workdays: 0,
                notes: None,
                contact_id: None,
                op_ref: None,
            }],
        };
        assert!(matches!(
            validate_patch(&p).unwrap_err(),
            ValidationError::NonPositiveDuration { .. }
        ));
    }

    #[test]
    fn rejects_unknown_dep_type() {
        let p = Patch {
            patch_version: 1,
            summary: "x".into(),
            ops: vec![PatchOp::AddDependency {
                predecessor: TaskRef::Existing { task_id: 1 },
                successor: TaskRef::Existing { task_id: 2 },
                dep_type: "XX".into(),
                lag_days: 0,
            }],
        };
        assert!(matches!(
            validate_patch(&p).unwrap_err(),
            ValidationError::BadDepType { .. }
        ));
    }

    #[test]
    fn rejects_dangling_op_ref() {
        let p = Patch {
            patch_version: 1,
            summary: "x".into(),
            ops: vec![PatchOp::AddDependency {
                predecessor: TaskRef::Existing { task_id: 1 },
                successor: TaskRef::Pending { op_ref: "ghost".into() },
                dep_type: "FS".into(),
                lag_days: 0,
            }],
        };
        assert!(matches!(
            validate_patch(&p).unwrap_err(),
            ValidationError::DanglingOpRef { .. }
        ));
    }

    #[test]
    fn accepts_valid_op_ref_chain() {
        let p = Patch {
            patch_version: 1,
            summary: "x".into(),
            ops: vec![
                PatchOp::AddTask {
                    phase_id: 1,
                    name: "a".into(),
                    start_date: "2026-06-03".into(),
                    duration_workdays: 1,
                    notes: None,
                    contact_id: None,
                    op_ref: Some("a".into()),
                },
                PatchOp::AddDependency {
                    predecessor: TaskRef::Pending { op_ref: "a".into() },
                    successor: TaskRef::Existing { task_id: 99 },
                    dep_type: "FS".into(),
                    lag_days: 0,
                },
            ],
        };
        assert!(validate_patch(&p).is_ok());
    }

    #[test]
    fn rejects_duplicate_op_ref() {
        let p = Patch {
            patch_version: 1,
            summary: "x".into(),
            ops: vec![
                PatchOp::AddTask {
                    phase_id: 1, name: "a".into(),
                    start_date: "2026-06-03".into(), duration_workdays: 1,
                    notes: None, contact_id: None,
                    op_ref: Some("dup".into()),
                },
                PatchOp::AddTask {
                    phase_id: 1, name: "b".into(),
                    start_date: "2026-06-03".into(), duration_workdays: 1,
                    notes: None, contact_id: None,
                    op_ref: Some("dup".into()),
                },
            ],
        };
        assert!(matches!(
            validate_patch(&p).unwrap_err(),
            ValidationError::DuplicateOpRef { .. }
        ));
    }
}
