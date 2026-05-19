//! Embedded skills and prompt templates.

/// Embedded skill.
#[derive(Debug, Clone, Copy)]
pub struct Skill {
    /// Stable skill name.
    pub name: &'static str,
    /// Resource URI.
    pub uri: &'static str,
    /// Markdown body.
    pub body: &'static str,
}

/// Embedded prompt.
#[derive(Debug, Clone, Copy)]
pub struct Prompt {
    /// Stable prompt name.
    pub name: &'static str,
    /// Resource URI.
    pub uri: &'static str,
    /// Prompt body.
    pub body: &'static str,
}

/// Returns all embedded skills.
pub fn skills() -> &'static [Skill] {
    &[
        Skill {
            name: "driftlock",
            uri: "driftlock://skills/driftlock",
            body: include_str!("../../../skills/driftlock/SKILL.md"),
        },
        Skill {
            name: "planner",
            uri: "driftlock://skills/planner",
            body: include_str!("../../../skills/driftlock-planner/SKILL.md"),
        },
        Skill {
            name: "worker",
            uri: "driftlock://skills/worker",
            body: include_str!("../../../skills/driftlock-worker/SKILL.md"),
        },
        Skill {
            name: "reviewer",
            uri: "driftlock://skills/reviewer",
            body: include_str!("../../../skills/driftlock-reviewer/SKILL.md"),
        },
        Skill {
            name: "maintainer",
            uri: "driftlock://skills/maintainer",
            body: include_str!("../../../skills/driftlock-maintainer/SKILL.md"),
        },
        Skill {
            name: "tdd",
            uri: "driftlock://skills/tdd",
            body: include_str!("../../../skills/driftlock-tdd/SKILL.md"),
        },
        Skill {
            name: "mcp-operator",
            uri: "driftlock://skills/mcp-operator",
            body: include_str!("../../../skills/driftlock-mcp-operator/SKILL.md"),
        },
    ]
}

/// Returns all embedded prompts.
pub fn prompts() -> &'static [Prompt] {
    &[
        Prompt {
            name: "driftlock.worker_start",
            uri: "driftlock://prompts/worker-start",
            body: include_str!("../../../prompts/worker-start.md"),
        },
        Prompt {
            name: "driftlock.planner_extract_adr",
            uri: "driftlock://prompts/planner-extract-adr",
            body: include_str!("../../../prompts/planner-extract-adr.md"),
        },
        Prompt {
            name: "driftlock.reviewer_gate",
            uri: "driftlock://prompts/reviewer-gate",
            body: include_str!("../../../prompts/reviewer-gate.md"),
        },
        Prompt {
            name: "driftlock.conflict_review",
            uri: "driftlock://prompts/conflict-review",
            body: include_str!("../../../prompts/conflict-review.md"),
        },
        Prompt {
            name: "driftlock.maintainer_refresh",
            uri: "driftlock://prompts/maintainer-refresh",
            body: include_str!("../../../prompts/maintainer-refresh.md"),
        },
        Prompt {
            name: "driftlock.agent_brief_template",
            uri: "driftlock://prompts/agent-brief-template",
            body: include_str!("../../../prompts/agent-brief-template.md"),
        },
    ]
}

/// Finds a skill by name or URI.
pub fn find_skill(name_or_uri: &str) -> Option<Skill> {
    skills().iter().copied().find(|skill| skill.name == name_or_uri || skill.uri == name_or_uri)
}

/// Finds a prompt by name or URI.
pub fn find_prompt(name_or_uri: &str) -> Option<Prompt> {
    prompts().iter().copied().find(|prompt| prompt.name == name_or_uri || prompt.uri == name_or_uri)
}

#[cfg(test)]
mod tests {
    #[test]
    fn worker_skill_is_embedded() {
        let worker = crate::find_skill("worker").expect("worker skill");
        assert!(
            worker.body.contains("Never implement from ADR prose")
                || worker.body.contains("Do not change")
        );
    }
}
