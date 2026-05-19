//! Schema export utilities.

use anyhow::Result;
use driftlock_core::schema;
use schemars::schema::RootSchema;
use std::fs;
use std::path::Path;

/// A named JSON Schema.
pub struct NamedSchema {
    /// File name.
    pub file_name: &'static str,
    /// Schema.
    pub schema: RootSchema,
}

/// Returns the schema bundle generated from Rust types.
pub fn schema_bundle() -> Vec<NamedSchema> {
    vec![
        NamedSchema {
            file_name: "work-order.generated.schema.json",
            schema: schema::work_order_schema(),
        },
        NamedSchema {
            file_name: "taskgraph.generated.schema.json",
            schema: schema::taskgraph_schema(),
        },
        NamedSchema {
            file_name: "lane-manifest.generated.schema.json",
            schema: schema::lane_manifest_schema(),
        },
        NamedSchema { file_name: "claim.generated.schema.json", schema: schema::claim_schema() },
        NamedSchema {
            file_name: "diff-report.generated.schema.json",
            schema: schema::diff_report_schema(),
        },
    ]
}

/// Writes generated schemas to a directory.
pub fn write_schemas(out_dir: impl AsRef<Path>) -> Result<()> {
    let out_dir = out_dir.as_ref();
    fs::create_dir_all(out_dir)?;
    for named in schema_bundle() {
        let path = out_dir.join(named.file_name);
        let json = serde_json::to_string_pretty(&named.schema)?;
        fs::write(path, json)?;
    }
    Ok(())
}
