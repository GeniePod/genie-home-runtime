use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpToolSpec {
    pub name: String,
    pub description: String,
    pub required_permissions: Vec<McpPermission>,
    pub input_schema: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpResourceSpec {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub required_permissions: Vec<McpPermission>,
    pub mime_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpPermission {
    HomeRead,
    HomeEvaluate,
    HomeActuate,
    HomeAuditRead,
    ConnectivityWrite,
    AutomationRun,
    SupportRead,
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
                vec![McpPermission::HomeRead],
                serde_json::json!({"type":"object","properties":{},"additionalProperties":false}),
            ),
            tool(
                "home.list_entities",
                "Return current home entity snapshots.",
                vec![McpPermission::HomeRead],
                serde_json::json!({"type":"object","properties":{},"additionalProperties":false}),
            ),
            tool(
                "home.list_services",
                "Return supported Home Assistant-style domain services.",
                vec![McpPermission::HomeRead],
                serde_json::json!({"type":"object","properties":{},"additionalProperties":false}),
            ),
            tool(
                "home.list_scenes",
                "Return registered scene definitions.",
                vec![McpPermission::HomeRead],
                serde_json::json!({"type":"object","properties":{},"additionalProperties":false}),
            ),
            tool(
                "home.list_automations",
                "Return registered automation definitions.",
                vec![McpPermission::HomeRead],
                serde_json::json!({"type":"object","properties":{},"additionalProperties":false}),
            ),
            tool(
                "home.evaluate",
                "Evaluate whether a physical home command is allowed without executing it.",
                vec![McpPermission::HomeEvaluate],
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
                vec![McpPermission::HomeActuate],
                serde_json::json!({
                    "type": "object",
                    "required": ["command"],
                    "properties": {"command": {"type": "object"}},
                    "additionalProperties": false
                }),
            ),
            tool(
                "home.call_service",
                "Call a Home Assistant-style domain service through Genie safety policy.",
                vec![McpPermission::HomeActuate],
                serde_json::json!({
                    "type": "object",
                    "required": ["call"],
                    "properties": {"call": {"type": "object"}},
                    "additionalProperties": false
                }),
            ),
            tool(
                "home.audit",
                "Return recent runtime safety and actuation decisions.",
                vec![McpPermission::HomeAuditRead],
                serde_json::json!({
                    "type": "object",
                    "properties": {"limit": {"type": "integer", "minimum": 1, "maximum": 200}},
                    "additionalProperties": false
                }),
            ),
            tool(
                "home.events",
                "Return recent runtime events such as state changes and service calls.",
                vec![McpPermission::HomeAuditRead],
                serde_json::json!({
                    "type": "object",
                    "properties": {"limit": {"type": "integer", "minimum": 1, "maximum": 500}},
                    "additionalProperties": false
                }),
            ),
            tool(
                "home.apply_connectivity_report",
                "Apply a GenieOS connectivity report for discovered devices.",
                vec![McpPermission::ConnectivityWrite],
                serde_json::json!({
                    "type": "object",
                    "required": ["report"],
                    "properties": {"report": {"type": "object"}},
                    "additionalProperties": false
                }),
            ),
            tool(
                "home.run_automation_tick",
                "Run local automation evaluation for a scheduler HH:MM tick.",
                vec![McpPermission::AutomationRun],
                serde_json::json!({
                    "type": "object",
                    "required": ["now_hh_mm"],
                    "properties": {"now_hh_mm": {"type": "string", "pattern": "^[0-2][0-9]:[0-5][0-9]$"}},
                    "additionalProperties": false
                }),
            ),
        ],
        resources: vec![
            resource(
                "genie-home://entities",
                "entities",
                "Current Genie Home Runtime entity graph snapshot.",
                vec![McpPermission::HomeRead],
            ),
            resource(
                "genie-home://services",
                "services",
                "Supported Home Assistant-style domain service catalog.",
                vec![McpPermission::HomeRead],
            ),
            resource(
                "genie-home://scenes",
                "scenes",
                "Registered Genie Home Runtime scene definitions.",
                vec![McpPermission::HomeRead],
            ),
            resource(
                "genie-home://automations",
                "automations",
                "Registered Genie Home Runtime automation definitions.",
                vec![McpPermission::HomeRead],
            ),
            resource(
                "genie-home://audit/recent",
                "recent_audit",
                "Recent physical-command safety decisions.",
                vec![McpPermission::HomeAuditRead],
            ),
            resource(
                "genie-home://events/recent",
                "recent_events",
                "Recent runtime events for state, services, connectivity, and automation.",
                vec![McpPermission::HomeAuditRead],
            ),
            resource(
                "genie-home://support-bundle",
                "support_bundle",
                "Local support diagnostics generated from persisted runtime files.",
                vec![McpPermission::SupportRead],
            ),
        ],
    }
}

fn tool(
    name: &str,
    description: &str,
    required_permissions: Vec<McpPermission>,
    input_schema: Value,
) -> McpToolSpec {
    McpToolSpec {
        name: name.into(),
        description: description.into(),
        required_permissions,
        input_schema,
    }
}

fn resource(
    uri: &str,
    name: &str,
    description: &str,
    required_permissions: Vec<McpPermission>,
) -> McpResourceSpec {
    McpResourceSpec {
        uri: uri.into(),
        name: name.into(),
        description: description.into(),
        required_permissions,
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
        let execute = surface
            .tools
            .iter()
            .find(|tool| tool.name == "home.execute")
            .unwrap();
        assert_eq!(
            execute.required_permissions,
            vec![McpPermission::HomeActuate]
        );
        assert!(surface.resources.iter().any(|resource| {
            resource.uri == "genie-home://audit/recent"
                && resource
                    .required_permissions
                    .contains(&McpPermission::HomeAuditRead)
        }));
    }
}
