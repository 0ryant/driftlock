//! Agent brief rendering.

use crate::model::{AcceptanceGate, WorkOrder};

/// Renders a bounded implementation brief.
pub fn render_agent_brief(task: &WorkOrder) -> String {
    format!(
        "# Work order: {id}\n\n\
## Objective\n{intent}\n\n\
## Source of truth\n- ADR: `{adr}`\n- Section: {section}\n- Lines: {start}-{end}\n- Evidence: {evidence}\n\n\
## Allowed writes\n{writes}\n\n\
## Read-only context\n{reads}\n\n\
## Must not change\n{non_goals}\n\n\
## Dependencies\n{deps}\n\n\
## Downstream unlocks\n{unlocks}\n\n\
## Acceptance\n{acceptance}\n",
        id = task.id.as_str(),
        intent = task.intent.as_str(),
        adr = task.source.adr.as_str(),
        section = task.source.section.as_str(),
        start = task.source.start_line,
        end = task.source.end_line,
        evidence = task.source.evidence.clone().unwrap_or_else(|| "No excerpt recorded".into()),
        writes = bullets(&task.write_set),
        reads = bullets(&task.read_set),
        non_goals = bullets(&task.non_goals),
        deps = bullets(&task.deps),
        unlocks = bullets(&task.unlocks),
        acceptance = acceptance_bullets(&task.acceptance),
    )
}

fn bullets(items: &[String]) -> String {
    if items.is_empty() {
        return "- None".to_string();
    }
    items.iter().map(|item| format!("- `{item}`")).collect::<Vec<_>>().join("\n")
}

/// Renders acceptance gates, marking each with how driftlock treats it so the
/// contract is honest: deterministic gates are `[driftlock-verified]`, command
/// obligations are `[delegated]`, and free-text gates are
/// `[advisory, unverified]`.
fn acceptance_bullets(gates: &[AcceptanceGate]) -> String {
    if gates.is_empty() {
        return "- None".to_string();
    }
    gates
        .iter()
        .map(|gate| match gate {
            AcceptanceGate::FileExists { file_exists } => {
                format!("- [driftlock-verified] file exists: `{file_exists}`")
            }
            AcceptanceGate::FileContains { file_contains, needle } => {
                format!("- [driftlock-verified] `{file_contains}` contains `{needle}`")
            }
            AcceptanceGate::Command { command } => {
                format!("- [delegated, not run by driftlock] `{command}`")
            }
            AcceptanceGate::Advisory(text) => {
                format!("- [advisory, unverified] {text}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
