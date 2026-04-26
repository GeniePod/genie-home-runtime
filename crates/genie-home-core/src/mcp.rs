use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpResourceSpec {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpSurface {
    pub name: String,
    pub version: String,
    pub tools: Vec<McpToolSpec>,
    pub resources: Vec<McpResourceSpec>,
}

pub fn default_mcp_surface() -> McpSurface {
    McpSurface {
        name: "genie-home-runtime".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        tools: vec![
            tool(
                "home.status",
                "Return runtime health, entity count, audit count, and safety policy summary.",
                serde_json::json!({"type":"object","properties":{},"additionalProperties":false}),
            ),
            tool(
                "home.list_entities",
                "Return current home entity snapshots.",
                serde_json::json!({"type":"object","properties":{},"additionalProperties":false}),
            ),
            tool(
                "home.list_scenes",
                "Return registered scene definitions.",
                serde_json::json!({"type":"object","properties":{},"additionalProperties":false}),
            ),
            tool(
                "home.evaluate",
                "Evaluate whether a physical home command is allowed without executing it.",
                serde_json::json!({
                    "type": "object",
                    "required": ["command"],
                    "properties": {"command": {"type": "object"}},
                    "additionalProperties": false
                }),
            ),
            tool(
                "home.execute",
                "Execute a physical home command only if the deterministic runtime safety policy allows it.",
                serde_json::json!({
                    "type": "object",
                    "required": ["command"],
                    "properties": {"command": {"type": "object"}},
                    "additionalProperties": false
                }),
            ),
            tool(
                "home.audit",
                "Return recent runtime safety and actuation decisions.",
                serde_json::json!({
                    "type": "object",
                    "properties": {"limit": {"type": "integer", "minimum": 1, "maximum": 200}},
                    "additionalProperties": false
                }),
            ),
            tool(
                "home.apply_connectivity_report",
                "Apply a GenieOS connectivity report for discovered devices.",
                serde_json::json!({
                    "type": "object",
                    "required": ["report"],
                    "properties": {"report": {"type": "object"}},
                    "additionalProperties": false
                }),
            ),
        ],
        resources: vec![
            resource(
                "genie-home://entities",
                "entities",
                "Current Genie Home Runtime entity graph snapshot.",
            ),
            resource(
                "genie-home://scenes",
                "scenes",
                "Registered Genie Home Runtime scene definitions.",
            ),
            resource(
                "genie-home://audit/recent",
                "recent_audit",
                "Recent physical-command safety decisions.",
            ),
            resource(
                "genie-home://support-bundle",
                "support_bundle",
                "Local support diagnostics generated from persisted runtime files.",
            ),
        ],
    }
}

fn tool(name: &str, description: &str, input_schema: Value) -> McpToolSpec {
    McpToolSpec {
        name: name.into(),
        description: description.into(),
        input_schema,
    }
}

fn resource(uri: &str, name: &str, description: &str) -> McpResourceSpec {
    McpResourceSpec {
        uri: uri.into(),
        name: name.into(),
        description: description.into(),
        mime_type: "application/json".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_surface_exposes_safety_first_tools() {
        let surface = default_mcp_surface();

        assert!(
            surface
                .tools
                .iter()
                .any(|tool| tool.name == "home.evaluate")
        );
        assert!(surface.tools.iter().any(|tool| tool.name == "home.execute"));
        assert!(
            surface
                .resources
                .iter()
                .any(|resource| resource.uri == "genie-home://audit/recent")
        );
    }
}
