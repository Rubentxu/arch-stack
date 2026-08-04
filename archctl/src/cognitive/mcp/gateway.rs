//! MCP gateway — read-only tool invocation over stdio.
//!
//! v1.0: hardcoded allowlist of 3 tools. No dynamic registration.
//! Input: JSON object `{tool: string, args: object}` from stdin.
//! Output: JSON object `{tool, error?, data?}` to stdout.

use serde::Deserialize;

use super::tools::{
    ToolResult, handle_graph_query, handle_run_tests_local, handle_schema_validate,
};

/// The 3 allowed tools in v1.0. No others.
pub const ALLOWED_TOOLS: &[&str] = &["graph_query", "schema_validate", "run_tests_local"];

/// MCP gateway that handles JSON-RPC-like requests from stdin.
#[derive(Default)]
pub struct McpGateway;

impl McpGateway {
    pub fn new() -> Self {
        Self
    }

    /// Handle a raw JSON request from stdin. Returns JSON response string.
    pub fn handle_raw(&self, input: &str) -> String {
        match self.handle_str(input) {
            Ok(result) => serde_json::to_string(&result)
                .unwrap_or_else(|e| serde_json::to_string(&ToolResult::err("mcp", e)).unwrap()),
            Err(e) => serde_json::to_string(&ToolResult::err("mcp", e)).unwrap(),
        }
    }

    fn handle_str(&self, input: &str) -> Result<ToolResult, McpError> {
        #[derive(Deserialize)]
        struct Request {
            tool: String,
            #[serde(default)]
            args: serde_json::Value,
        }

        let req: Request =
            serde_json::from_str(input).map_err(|e| McpError::ParseError(e.to_string()))?;

        if !ALLOWED_TOOLS.contains(&req.tool.as_str()) {
            return Err(McpError::ToolNotAllowed(req.tool));
        }

        let result = match req.tool.as_str() {
            "graph_query" => {
                let args = serde_json::from_value(req.args)
                    .map_err(|e| McpError::InvalidArgs(e.to_string()))?;
                handle_graph_query(args)
            }
            "schema_validate" => {
                let args = serde_json::from_value(req.args)
                    .map_err(|e| McpError::InvalidArgs(e.to_string()))?;
                handle_schema_validate(args)
            }
            "run_tests_local" => {
                let args = serde_json::from_value(req.args)
                    .map_err(|e| McpError::InvalidArgs(e.to_string()))?;
                handle_run_tests_local(args)
            }
            _ => unreachable!(),
        };

        Ok(result)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("invalid JSON: {0}")]
    ParseError(String),
    #[error("tool not in allowlist: {0}")]
    ToolNotAllowed(String),
    #[error("invalid tool arguments: {0}")]
    InvalidArgs(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_allows_graph_query() {
        let gw = McpGateway::new();
        let req = r#"{"tool":"graph_query","args":{"cypher":"MATCH (e) RETURN e","params":{}}}"#;
        let out = gw.handle_raw(req);
        let result: ToolResult = serde_json::from_str(&out).unwrap();
        // graph_query without a db will error, but it's allowed
        assert_eq!(result.tool, "graph_query");
    }

    #[test]
    fn gateway_denies_unknown_tool() {
        let gw = McpGateway::new();
        let req = r#"{"tool":"delete_everything","args":{}}"#;
        let out = gw.handle_raw(req);
        let result: ToolResult = serde_json::from_str(&out).unwrap();
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("not in allowlist"));
    }

    #[test]
    fn gateway_rejects_malformed_json() {
        let gw = McpGateway::new();
        let out = gw.handle_raw("not json at all");
        let result: ToolResult = serde_json::from_str(&out).unwrap();
        assert!(result.error.is_some());
    }
}
