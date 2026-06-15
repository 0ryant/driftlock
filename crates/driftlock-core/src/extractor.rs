//! Deterministic ADR obligation extraction.

use crate::adr::{find_section, sections};
use crate::model::{AcceptanceGate, Confidence, EvidenceSpan, LaneManifest, TaskStatus, WorkOrder};
use crate::Result;
use regex::Regex;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Loads a lane manifest TOML file.
pub fn load_lane_manifest(path: impl AsRef<Path>) -> Result<LaneManifest> {
    let text = fs::read_to_string(path.as_ref())?;
    Ok(toml::from_str(&text)?)
}

/// Extracts candidate work orders from ADR Markdown with lane-aware write sets.
pub fn extract_work_orders_from_adr(
    adr_path: &str,
    adr_revision: &str,
    markdown: &str,
    lane: &str,
    lanes: Option<&LaneManifest>,
) -> Vec<WorkOrder> {
    let (write_set, read_set, exclusive) = lane_bounds(lane, lanes);
    let file_scope = if write_set.is_empty() { 0.30 } else { 0.85 };

    let sections = sections(markdown);
    let Some(obligations) = find_section(&sections, "Obligations") else {
        return Vec::new();
    };

    let adr_id = extract_adr_id(adr_path).unwrap_or_else(|| "adr-0000".to_string());
    let bullet_re = Regex::new(r"^\s*[-*]\s+(?P<body>.+?)\s*$").expect("valid regex");
    let bullets: Vec<(u32, String)> = obligations
        .body
        .lines()
        .enumerate()
        .filter_map(|(offset, line)| {
            let body = bullet_re
                .captures(line)?
                .name("body")?
                .as_str()
                .trim()
                .trim_end_matches('.')
                .to_string();
            let line_no = obligations
                .start_line
                .saturating_add(u32::try_from(offset).unwrap_or(0))
                .saturating_add(1);
            Some((line_no, body))
        })
        .collect();

    bullets
        .into_iter()
        .enumerate()
        .map(|(idx, (line_no, body))| {
            let task_no = idx + 1;
            let id = format!("{adr_id}:T{task_no:02}");
            WorkOrder {
                id,
                title: body.clone(),
                source: EvidenceSpan {
                    adr: adr_path.to_string(),
                    adr_revision: adr_revision.to_string(),
                    section: "Obligations".to_string(),
                    start_line: line_no,
                    end_line: line_no,
                    evidence: Some(body.clone()),
                },
                intent: format!("Deliver ADR obligation: {body}."),
                lane: lane.to_string(),
                status: TaskStatus::NeedsReview,
                write_set: write_set.clone(),
                read_set: read_set.clone(),
                exclusive_resources: exclusive.clone(),
                deps: Vec::new(),
                unlocks: Vec::new(),
                conflicts: Vec::new(),
                acceptance: vec![AcceptanceGate::Advisory(
                    "Define acceptance gates before marking ready.".to_string(),
                )],
                non_goals: vec![
                    "Do not infer unrelated implementation work from ADR prose.".to_string()
                ],
                confidence: Confidence {
                    task_extraction: 0.82,
                    file_scope,
                    dependency_edges: 0.30,
                },
                metadata: BTreeMap::new(),
            }
        })
        .collect()
}

fn lane_bounds(
    lane: &str,
    manifest: Option<&LaneManifest>,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let Some(manifest) = manifest else {
        return (Vec::new(), vec!["docs/adrs/**".into()], Vec::new());
    };
    let Some(l) = manifest.lanes.iter().find(|l| l.id == lane) else {
        return (Vec::new(), Vec::new(), Vec::new());
    };
    (l.write_allow.clone(), l.read_allow.clone(), l.exclusive.clone())
}

fn extract_adr_id(path: &str) -> Option<String> {
    let re = Regex::new(r"(?P<num>[0-9]{4})").ok()?;
    let num = re.captures(path)?.name("num")?.as_str();
    Some(format!("adr-{num}"))
}

#[cfg(test)]
mod tests {
    use super::{extract_work_orders_from_adr, load_lane_manifest};
    use crate::model::TaskStatus;

    #[test]
    fn extracts_obligation_bullets() {
        let md = "# ADR-0009\n\n## Obligations\n\n- Define schema first.\n- Add tests.\n";
        let tasks =
            extract_work_orders_from_adr("docs/adrs/0009-test.md", "head", md, "core", None);
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id, "adr-0009:T01");
        assert_eq!(tasks[0].status, TaskStatus::NeedsReview);
    }

    #[test]
    fn infers_write_set_from_lanes() {
        let lanes_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lanes/default.toml");
        let lanes = load_lane_manifest(lanes_path).expect("lanes");
        let md = "# ADR-0001\n\n## Obligations\n\n- Do thing.\n";
        let tasks =
            extract_work_orders_from_adr("docs/adrs/0001-x.md", "abc", md, "core", Some(&lanes));
        assert!(!tasks[0].write_set.is_empty());
        assert!(tasks[0].confidence.file_scope >= 0.75);
    }
}
