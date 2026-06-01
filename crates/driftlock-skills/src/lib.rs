//! Embedded skills and prompt templates.

mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}

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
            body: generated::SKILL_DRIFTLOCK,
        },
        Skill {
            name: "planner",
            uri: "driftlock://skills/planner",
            body: generated::SKILL_PLANNER,
        },
        Skill { name: "worker", uri: "driftlock://skills/worker", body: generated::SKILL_WORKER },
        Skill {
            name: "reviewer",
            uri: "driftlock://skills/reviewer",
            body: generated::SKILL_REVIEWER,
        },
        Skill {
            name: "maintainer",
            uri: "driftlock://skills/maintainer",
            body: generated::SKILL_MAINTAINER,
        },
        Skill { name: "tdd", uri: "driftlock://skills/tdd", body: generated::SKILL_TDD },
        Skill {
            name: "mcp-operator",
            uri: "driftlock://skills/mcp-operator",
            body: generated::SKILL_MCP_OPERATOR,
        },
    ]
}

/// Returns all embedded prompts.
pub fn prompts() -> &'static [Prompt] {
    &[
        Prompt {
            name: "driftlock.worker_start",
            uri: "driftlock://prompts/worker-start",
            body: generated::PROMPT_WORKER_START,
        },
        Prompt {
            name: "driftlock.planner_extract_adr",
            uri: "driftlock://prompts/planner-extract-adr",
            body: generated::PROMPT_PLANNER_EXTRACT_ADR,
        },
        Prompt {
            name: "driftlock.reviewer_gate",
            uri: "driftlock://prompts/reviewer-gate",
            body: generated::PROMPT_REVIEWER_GATE,
        },
        Prompt {
            name: "driftlock.conflict_review",
            uri: "driftlock://prompts/conflict-review",
            body: generated::PROMPT_CONFLICT_REVIEW,
        },
        Prompt {
            name: "driftlock.maintainer_refresh",
            uri: "driftlock://prompts/maintainer-refresh",
            body: generated::PROMPT_MAINTAINER_REFRESH,
        },
        Prompt {
            name: "driftlock.agent_brief_template",
            uri: "driftlock://prompts/agent-brief-template",
            body: generated::PROMPT_AGENT_BRIEF_TEMPLATE,
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
