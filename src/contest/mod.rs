//! Contest definition format (v0.3) — types, parser, and validator.
//!
//! See `docs/features/contest-definitions.md` for the canonical format
//! reference.
//!
//! This module exposes the data types ([`ContestDefinition`], [`Contest`],
//! [`Phase`], etc.) along with a semantic validation pass
//! ([`ContestDefinition::validate`]). Loading and validating a definition is
//! a two-step process:
//!
//! ```ignore
//! let def: ContestDefinition = serde_json::from_str(json)?;
//! let problems = def.validate();
//! if !def.is_valid() {
//!     // surface problems to the user
//! }
//! ```
//!
//! Scoring engine, dupe checking, and Cabrillo export are deferred to a
//! later pass.

#![allow(dead_code, unused_imports)]

pub mod types;
pub mod validation;

pub use types::*;
pub use validation::{Severity, ValidationError};

#[cfg(test)]
mod tests {
    use super::*;

    fn load(name: &str) -> ContestDefinition {
        let path = format!("docs/examples/contests/{name}");
        let bytes = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
        serde_json::from_str(&bytes)
            .unwrap_or_else(|e| panic!("failed to parse {path}: {e}"))
    }

    fn assert_valid(def: &ContestDefinition) {
        let problems = def.validate();
        let errors: Vec<_> = problems
            .iter()
            .filter(|p| p.severity == Severity::Error)
            .collect();
        if !errors.is_empty() {
            panic!(
                "expected no errors, got:\n{}",
                errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
    }

    #[test]
    fn parses_cwt_example() {
        let def = load("cwt.json");
        assert_eq!(def.version, "0.3.0");
        assert_eq!(def.contests.len(), 1);
        let cwt = &def.contests[0];
        assert_eq!(cwt.id, "cwops-cwt");
        assert!(matches!(cwt.schedule, Schedule::RecurringWeekly { .. }));
        assert_valid(&def);
    }

    #[test]
    fn parses_sst_example() {
        let def = load("sst.json");
        assert_valid(&def);
    }

    #[test]
    fn parses_mst_example() {
        let def = load("mst.json");
        assert_valid(&def);
        // MST uses an auto-incrementing serial on the sent side.
        let mst = &def.contests[0];
        let serial = mst
            .exchange
            .sent
            .iter()
            .find(|f| f.field_type == "serial_number")
            .expect("MST has a serial_number sent field");
        assert_eq!(serial.auto_increment, Some(true));
    }

    #[test]
    fn parses_cqww_example() {
        let def = load("cqww-cw.json");
        assert_valid(&def);
        let cqww = &def.contests[0];
        assert!(matches!(cqww.schedule, Schedule::Annual { .. }));
        assert!(cqww.dupe_rules.zero_point_qsos_count_for_mults);
        assert_eq!(cqww.scoring.phases.len(), 4);
    }

    #[test]
    fn parses_combined_bundle() {
        let def = load("sample-bundle.json");
        assert_eq!(def.contests.len(), 4);
        assert_valid(&def);
    }

    #[test]
    fn round_trip_serialize() {
        let def = load("cqww-cw.json");
        let serialized = serde_json::to_string(&def).expect("serialize");
        let again: ContestDefinition =
            serde_json::from_str(&serialized).expect("re-deserialize");
        assert_eq!(again.contests.len(), def.contests.len());
        assert_valid(&again);
    }

    // ---------- negative cases ----------

    #[test]
    fn rejects_unknown_band() {
        let def = load("cwt.json");
        let mut def = def;
        def.contests[0].bands.push("99m".to_string());
        let problems = def.validate();
        assert!(problems
            .iter()
            .any(|p| p.severity == Severity::Error && p.path.contains("bands")));
    }

    #[test]
    fn rejects_unknown_mode() {
        let def = load("cwt.json");
        let mut def = def;
        def.contests[0].modes.push("MAGIC".to_string());
        let problems = def.validate();
        assert!(problems
            .iter()
            .any(|p| p.severity == Severity::Error && p.path.contains("modes")));
    }

    #[test]
    fn rejects_forward_phase_ref() {
        // Build a definition where the aggregate references a later phase.
        let json = r#"{
            "version": "0.3.0",
            "contests": [{
                "id": "bad",
                "name": "Bad",
                "modes": ["CW"],
                "bands": ["20m"],
                "schedule": {
                    "type": "recurring_weekly",
                    "sessions": [{ "day": "monday", "start_utc": "00:00", "duration_minutes": 60 }],
                    "session_scoring": "independent"
                },
                "exchange": { "sent": [], "received": [] },
                "dupe_rules": { "scope": "session", "key": ["callsign", "band"] },
                "scoring": {
                    "phases": [
                        { "id": "total", "type": "aggregate",
                          "operation": { "op": "ref", "phase": "qso_points" } },
                        { "id": "qso_points", "type": "per_qso",
                          "rules": [{ "value": 1 }] }
                    ]
                },
                "categories": [{ "id": "single_op", "label": "Single Op" }]
            }]
        }"#;
        let def: ContestDefinition = serde_json::from_str(json).unwrap();
        let problems = def.validate();
        assert!(
            problems
                .iter()
                .any(|p| p.severity == Severity::Error && p.message.contains("not defined earlier")),
            "expected forward-ref error, got {problems:?}"
        );
    }

    #[test]
    fn rejects_unknown_condition_field() {
        let json = r#"{
            "version": "0.3.0",
            "contests": [{
                "id": "bad",
                "name": "Bad",
                "modes": ["CW"],
                "bands": ["20m"],
                "schedule": {
                    "type": "recurring_weekly",
                    "sessions": [{ "day": "monday", "start_utc": "00:00", "duration_minutes": 60 }],
                    "session_scoring": "independent"
                },
                "exchange": { "sent": [], "received": [] },
                "dupe_rules": { "scope": "session", "key": ["callsign", "band"] },
                "scoring": {
                    "phases": [{
                        "id": "qso_points", "type": "per_qso",
                        "rules": [{
                            "conditions": [
                                { "field": "their_unicorn", "op": "eq", "value": "x" }
                            ],
                            "value": 1
                        }]
                    }]
                },
                "categories": [{ "id": "single_op", "label": "Single Op" }]
            }]
        }"#;
        let def: ContestDefinition = serde_json::from_str(json).unwrap();
        let problems = def.validate();
        assert!(problems
            .iter()
            .any(|p| p.severity == Severity::Error && p.message.contains("their_unicorn")));
    }

    #[test]
    fn rejects_received_serial_with_auto_increment() {
        let json = r#"{
            "version": "0.3.0",
            "contests": [{
                "id": "bad",
                "name": "Bad",
                "modes": ["CW"],
                "bands": ["20m"],
                "schedule": {
                    "type": "recurring_weekly",
                    "sessions": [{ "day": "monday", "start_utc": "00:00", "duration_minutes": 60 }],
                    "session_scoring": "independent"
                },
                "exchange": {
                    "sent": [],
                    "received": [
                        { "name": "serial", "type": "serial_number", "label": "S#",
                          "auto_increment": true }
                    ]
                },
                "dupe_rules": { "scope": "session", "key": ["callsign", "band"] },
                "scoring": {
                    "phases": [{ "id": "qp", "type": "per_qso", "rules": [{ "value": 1 }] }]
                },
                "categories": [{ "id": "single_op", "label": "Single Op" }]
            }]
        }"#;
        let def: ContestDefinition = serde_json::from_str(json).unwrap();
        let problems = def.validate();
        assert!(problems
            .iter()
            .any(|p| p.severity == Severity::Error && p.message.contains("sent side")));
    }

    #[test]
    fn warns_when_last_rule_has_conditions() {
        let json = r#"{
            "version": "0.3.0",
            "contests": [{
                "id": "warn",
                "name": "Warn",
                "modes": ["CW"],
                "bands": ["20m"],
                "schedule": {
                    "type": "recurring_weekly",
                    "sessions": [{ "day": "monday", "start_utc": "00:00", "duration_minutes": 60 }],
                    "session_scoring": "independent"
                },
                "exchange": { "sent": [], "received": [] },
                "dupe_rules": { "scope": "session", "key": ["callsign", "band"] },
                "scoring": {
                    "phases": [{
                        "id": "qp", "type": "per_qso", "rules": [{
                            "conditions": [
                                { "field": "band", "op": "eq", "value": "20m" }
                            ],
                            "value": 1
                        }]
                    }]
                },
                "categories": [{ "id": "single_op", "label": "Single Op" }]
            }]
        }"#;
        let def: ContestDefinition = serde_json::from_str(json).unwrap();
        let problems = def.validate();
        assert!(problems
            .iter()
            .any(|p| p.severity == Severity::Warning && p.message.contains("silently score 0")));
    }
}
