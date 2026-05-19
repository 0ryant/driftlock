//! ADR parsing helpers.

/// A markdown section with its line range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdrSection {
    /// Heading text without leading hashes.
    pub title: String,
    /// Start line, one-indexed.
    pub start_line: u32,
    /// End line, one-indexed.
    pub end_line: u32,
    /// Section body.
    pub body: String,
}

/// Splits markdown into top-level and nested heading sections.
pub fn sections(markdown: &str) -> Vec<AdrSection> {
    let mut out = Vec::new();
    let mut current_title = String::from("Preamble");
    let mut current_start = 1_u32;
    let mut current_body = Vec::<String>::new();

    for (idx, line) in markdown.lines().enumerate() {
        let line_no = u32::try_from(idx + 1).unwrap_or(u32::MAX);
        if line.starts_with("## ") {
            if !current_body.is_empty() || current_title != "Preamble" {
                out.push(AdrSection {
                    title: current_title,
                    start_line: current_start,
                    end_line: line_no.saturating_sub(1),
                    body: current_body.join("\n"),
                });
            }
            current_title = line.trim_start_matches('#').trim().to_string();
            current_start = line_no;
            current_body.clear();
        } else {
            current_body.push(line.to_string());
        }
    }

    let total = u32::try_from(markdown.lines().count().max(1)).unwrap_or(u32::MAX);
    out.push(AdrSection {
        title: current_title,
        start_line: current_start,
        end_line: total,
        body: current_body.join("\n"),
    });
    out
}

/// Returns the first section whose title matches case-insensitively.
pub fn find_section<'a>(sections: &'a [AdrSection], title: &str) -> Option<&'a AdrSection> {
    sections.iter().find(|s| s.title.eq_ignore_ascii_case(title))
}
