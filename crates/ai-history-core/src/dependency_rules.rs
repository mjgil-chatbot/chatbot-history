//! Dependency boundary enforcement for ai-history crates
//!
//! Task 3: Add dependency-boundary rule:
//! - `core` has no storage/runtime dependencies
//! - `fsm` is pure (no filesystem/database calls)
//! - `events` owns event schema
//! - `store` owns persistence
//! - `restore` owns workspace mutation
//! - `ui-model` owns read DTOs only

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CrateName {
    Core,
    Fsm,
    Events,
    Store,
    Snapshot,
    Diff,
    Restore,
    Context,
    UiModel,
    Cli,
    Testkit,
}

impl CrateName {
    pub fn as_str(&self) -> &'static str {
        match self {
            CrateName::Core => "ai-history-core",
            CrateName::Fsm => "ai-history-fsm",
            CrateName::Events => "ai-history-events",
            CrateName::Store => "ai-history-store",
            CrateName::Snapshot => "ai-history-snapshot",
            CrateName::Diff => "ai-history-diff",
            CrateName::Restore => "ai-history-restore",
            CrateName::Context => "ai-history-context",
            CrateName::UiModel => "ai-history-ui-model",
            CrateName::Cli => "ai-history-cli",
            CrateName::Testkit => "ai-history-testkit",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ai-history-core" => Some(CrateName::Core),
            "ai-history-fsm" => Some(CrateName::Fsm),
            "ai-history-events" => Some(CrateName::Events),
            "ai-history-store" => Some(CrateName::Store),
            "ai-history-snapshot" => Some(CrateName::Snapshot),
            "ai-history-diff" => Some(CrateName::Diff),
            "ai-history-restore" => Some(CrateName::Restore),
            "ai-history-context" => Some(CrateName::Context),
            "ai-history-ui-model" => Some(CrateName::UiModel),
            "ai-history-cli" => Some(CrateName::Cli),
            "ai-history-testkit" => Some(CrateName::Testkit),
            _ => None,
        }
    }
}

/// Dependency rule type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyRule {
    /// This crate may depend on the target
    Allowed,
    /// This crate may NOT depend on the target
    Forbidden,
    /// Only these specific features may enable this dependency
    FeatureGated { features: Vec<String> },
}

/// A dependency edge between crates
#[derive(Debug, Clone)]
pub struct DependencyEdge {
    pub from: CrateName,
    pub to: CrateName,
    pub is_optional: bool,
    pub features: Vec<String>,
}

/// A rule violation
#[derive(Debug, Clone)]
pub struct DependencyViolation {
    pub from: CrateName,
    pub to: CrateName,
    pub rule: String,
    pub explanation: String,
}

/// Dependency rules for the workspace
pub struct DependencyRules {
    rules: HashMap<(CrateName, CrateName), DependencyRule>,
}

impl Default for DependencyRules {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyRules {
    pub fn new() -> Self {
        let mut rules = HashMap::new();

        // Core should have minimal dependencies
        // Core may NOT depend on store (storage-specific)
        rules.insert(
            (CrateName::Core, CrateName::Store),
            DependencyRule::Forbidden,
        );
        rules.insert(
            (CrateName::Core, CrateName::Restore),
            DependencyRule::Forbidden,
        );
        rules.insert((CrateName::Core, CrateName::Cli), DependencyRule::Forbidden);

        // FSM must be pure - may NOT depend on store, snapshot, restore, cli
        rules.insert(
            (CrateName::Fsm, CrateName::Store),
            DependencyRule::Forbidden,
        );
        rules.insert(
            (CrateName::Fsm, CrateName::Snapshot),
            DependencyRule::Forbidden,
        );
        rules.insert(
            (CrateName::Fsm, CrateName::Restore),
            DependencyRule::Forbidden,
        );
        rules.insert((CrateName::Fsm, CrateName::Cli), DependencyRule::Forbidden);

        // Events owns event schema - no runtime deps
        rules.insert(
            (CrateName::Events, CrateName::Store),
            DependencyRule::Forbidden,
        );
        rules.insert(
            (CrateName::Events, CrateName::Restore),
            DependencyRule::Forbidden,
        );

        // UI model only owns read DTOs - no storage, no restore
        rules.insert(
            (CrateName::UiModel, CrateName::Store),
            DependencyRule::Forbidden,
        );
        rules.insert(
            (CrateName::UiModel, CrateName::Restore),
            DependencyRule::Forbidden,
        );
        rules.insert(
            (CrateName::UiModel, CrateName::Cli),
            DependencyRule::Forbidden,
        );

        // Testkit can depend on everything (it's for testing)
        // Context can depend on core and events

        Self { rules }
    }

    /// Check if a dependency is allowed
    pub fn is_allowed(&self, from: CrateName, to: CrateName) -> Option<&DependencyRule> {
        self.rules.get(&(from, to))
    }

    /// Get all allowed dependencies for a crate
    pub fn allowed_deps(&self, _from: CrateName) -> Vec<CrateName> {
        vec![
            CrateName::Core,
            CrateName::Fsm,
            CrateName::Events,
            CrateName::Store,
            CrateName::Snapshot,
            CrateName::Diff,
            CrateName::Restore,
            CrateName::Context,
            CrateName::UiModel,
            CrateName::Cli,
            CrateName::Testkit,
        ]
    }
}

/// Check a dependency graph for violations
pub struct DependencyChecker {
    rules: DependencyRules,
}

impl DependencyChecker {
    pub fn new() -> Self {
        Self {
            rules: DependencyRules::new(),
        }
    }

    /// Check if a dependency violates rules
    pub fn check_dependency(&self, from: CrateName, to: CrateName) -> Option<DependencyViolation> {
        if let Some(rule) = self.rules.is_allowed(from, to) {
            match rule {
                DependencyRule::Forbidden => Some(DependencyViolation {
                    from,
                    to,
                    rule: "Forbidden".to_string(),
                    explanation: format!("{} may not depend on {}", from.as_str(), to.as_str()),
                }),
                _ => None,
            }
        } else {
            // No rule means allowed
            None
        }
    }

    /// Check a full dependency list
    pub fn check_all(
        &self,
        dependencies: &[(String, String)], // (from_crate, to_crate)
    ) -> Vec<DependencyViolation> {
        let mut violations = Vec::new();

        for (from_str, to_str) in dependencies {
            if let (Some(from), Some(to)) =
                (CrateName::from_str(from_str), CrateName::from_str(to_str))
            {
                if let Some(violation) = self.check_dependency(from, to) {
                    violations.push(violation);
                }
            }
        }

        violations
    }

    /// Check for cycles in the dependency graph
    pub fn find_cycles(&self, edges: &[(String, String)]) -> Vec<Vec<String>> {
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut all_nodes: HashSet<String> = HashSet::new();

        for (from, to) in edges {
            graph.entry(from.clone()).or_default().push(to.clone());
            all_nodes.insert(from.clone());
            all_nodes.insert(to.clone());
        }

        let mut cycles = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut rec_stack: HashSet<String> = HashSet::new();
        let mut path: Vec<String> = Vec::new();

        fn dfs(
            node: &str,
            graph: &HashMap<String, Vec<String>>,
            visited: &mut HashSet<String>,
            rec_stack: &mut HashSet<String>,
            path: &mut Vec<String>,
            cycles: &mut Vec<Vec<String>>,
        ) {
            visited.insert(node.to_string());
            rec_stack.insert(node.to_string());
            path.push(node.to_string());

            if let Some(neighbors) = graph.get(node) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        dfs(neighbor, graph, visited, rec_stack, path, cycles);
                    } else if rec_stack.contains(neighbor) {
                        // Found cycle
                        if let Some(pos) = path.iter().position(|n| n == neighbor) {
                            let cycle: Vec<String> = path[pos..].to_vec();
                            cycles.push(cycle);
                        }
                    }
                }
            }

            path.pop();
            rec_stack.remove(node);
        }

        for node in &all_nodes {
            if !visited.contains(node) {
                dfs(
                    node,
                    &graph,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }

        cycles
    }
}

impl Default for DependencyChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse Cargo.toml dependencies section
pub fn parse_cargo_deps(toml_content: &str) -> Vec<(String, Vec<String>)> {
    let mut deps = Vec::new();

    // Simple regex-like parsing for cargo dependencies
    // In real implementation, would use a proper TOML parser
    for line in toml_content.lines() {
        if line.trim().starts_with("ai-history-") && line.contains("=") {
            let parts: Vec<&str> = line.split('=').collect();
            if parts.len() >= 1 {
                let crate_name = parts[0].trim().trim_matches('"').trim();
                deps.push((crate_name.to_string(), Vec::new()));
            }
        }
    }

    deps
}

/// Cargo.toml dependency entry
#[derive(Debug, Clone)]
pub struct CargoDependency {
    pub name: String,
    pub version: Option<String>,
    pub path: Option<String>,
    pub features: Vec<String>,
    pub optional: bool,
}

/// Parse a single dependency line from Cargo.toml
fn parse_dependency_line(line: &str) -> Option<CargoDependency> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    // Match pattern: crate_name = { ... } or crate_name = "version"
    // We handle both simple and table-style dependency declarations
    if let Some(eq_pos) = line.find(" = ") {
        let name = line[..eq_pos].trim().to_string();

        let rest = line[eq_pos + 3..].trim();

        // Simple version string: "1.0" or workspace = true
        if rest.starts_with('"') || rest.starts_with("workspace") {
            if rest.starts_with('"') {
                let version = rest.trim_matches('"').to_string();
                return Some(CargoDependency {
                    name,
                    version: Some(version),
                    path: None,
                    features: Vec::new(),
                    optional: false,
                });
            }
            // Handle workspace = true style
            return Some(CargoDependency {
                name,
                version: Some(rest.to_string()),
                path: None,
                features: Vec::new(),
                optional: false,
            });
        }

        // Table style: { version = "...", features = [...] }
        if rest.starts_with('{') {
            let mut version = None;
            let mut path = None;
            let mut features = Vec::new();
            let mut optional = false;

            // Remove surrounding braces
            let inner = rest.trim_start_matches('{').trim_end_matches('}');

            // Parse key-value pairs - need to track bracket depth when splitting by comma
            let mut part_start = 0;
            let mut bracket_depth = 0;
            let mut in_string = false;
            let mut parts: Vec<&str> = Vec::new();

            for (i, ch) in inner.char_indices() {
                match ch {
                    '"' => {
                        in_string = !in_string;
                    }
                    '[' | '{' if !in_string => {
                        bracket_depth += 1;
                    }
                    ']' | '}' if !in_string => {
                        bracket_depth -= 1;
                    }
                    ',' if !in_string && bracket_depth == 0 => {
                        parts.push(inner[part_start..i].trim());
                        part_start = i + 1;
                    }
                    _ => {}
                }
            }

            // Don't forget the last part
            if part_start < inner.len() {
                parts.push(inner[part_start..].trim());
            }

            for part in parts {
                if let Some(eq_pos) = part.find(" = ") {
                    let key = part[..eq_pos].trim();
                    let value = part[eq_pos + 3..].trim();

                    match key {
                        "version" => {
                            version = Some(value.trim_matches('"').to_string());
                        }
                        "path" => {
                            path = Some(value.trim_matches('"').to_string());
                        }
                        "workspace" => {
                            version = Some(value.trim_matches('"').to_string());
                        }
                        "features" => {
                            // Parse features array - handle nested brackets by counting depth
                            if value.starts_with('[') {
                                let mut depth = 0i32;
                                let mut in_string = false;
                                let mut current_feat = String::new();

                                for ch in value.chars() {
                                    match ch {
                                        '"' => {
                                            // Toggle string state, don't include quote in content
                                            in_string = !in_string;
                                        }
                                        '[' if !in_string => {
                                            depth += 1;
                                        }
                                        ']' if !in_string => {
                                            if depth == 1 {
                                                let feat = current_feat.trim().trim_matches('"');
                                                if !feat.is_empty() {
                                                    features.push(feat.to_string());
                                                }
                                                current_feat.clear();
                                            }
                                            depth = (depth - 1).max(0);
                                            if depth > 0 {
                                                current_feat.push(ch);
                                            }
                                        }
                                        ',' if !in_string && depth == 1 => {
                                            let feat = current_feat.trim().trim_matches('"');
                                            if !feat.is_empty() {
                                                features.push(feat.to_string());
                                            }
                                            current_feat.clear();
                                        }
                                        _ if depth >= 1 && in_string => {
                                            // Only push content when inside quotes
                                            current_feat.push(ch);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        "optional" => {
                            optional = value.trim_matches('"') == "true";
                        }
                        _ => {}
                    }
                }
            }

            return Some(CargoDependency {
                name,
                version,
                path,
                features,
                optional,
            });
        }
    }

    None
}

/// Parse all dependencies from a Cargo.toml content string
pub fn parse_cargo_toml_dependencies(toml_content: &str) -> Vec<CargoDependency> {
    let mut deps = Vec::new();
    let mut in_dependencies = false;

    for line in toml_content.lines() {
        let trimmed = line.trim();

        // Check for [dependencies] section header
        if trimmed.starts_with('[') {
            in_dependencies = trimmed == "[dependencies]"
                || trimmed == "[dev-dependencies]"
                || trimmed == "[build-dependencies]";
            continue;
        }

        if in_dependencies {
            if let Some(dep) = parse_dependency_line(line) {
                deps.push(dep);
            }
        }
    }

    deps
}

/// Load and parse a Cargo.toml file, returning workspace crate dependencies
pub fn load_cargo_dependencies(manifest_path: &Path) -> Result<Vec<(String, String)>, String> {
    let content = fs::read_to_string(manifest_path)
        .map_err(|e| format!("Failed to read Cargo.toml: {}", e))?;

    let deps = parse_cargo_toml_dependencies(&content);

    // Filter to only ai-history crates and convert to (from_crate, to_crate) pairs
    // The from_crate is determined by the manifest path
    let crate_name = manifest_path
        .file_stem()
        .and_then(|n| n.to_str())
        .ok_or("Invalid manifest path")?
        .to_string();

    let mut pairs = Vec::new();
    for dep in deps {
        if dep.name.starts_with("ai-history-") {
            pairs.push((crate_name.clone(), dep.name));
        }
    }

    Ok(pairs)
}

/// CI-enforced dependency graph parser
/// Parses all Cargo.toml files in the workspace to build the full dependency graph
pub struct WorkspaceDependencyGraph {
    pub edges: Vec<(String, String)>, // (from, to)
    pub crate_paths: HashMap<String, String>,
}

impl WorkspaceDependencyGraph {
    /// Build the full dependency graph from workspace Cargo.toml files
    pub fn build_from_workspace(workspace_root: &Path) -> Result<Self, String> {
        let mut edges = Vec::new();
        let mut crate_paths = HashMap::new();

        // Read workspace Cargo.toml to get crate list
        let workspace_toml = workspace_root.join("Cargo.toml");
        let content = fs::read_to_string(&workspace_toml)
            .map_err(|e| format!("Failed to read workspace Cargo.toml: {}", e))?;

        // Extract workspace members
        let members = Self::extract_workspace_members(&content)?;

        // Parse each crate's Cargo.toml
        for member in members {
            let crate_dir = workspace_root.join(&member);
            let manifest_path = crate_dir.join("Cargo.toml");

            if manifest_path.exists() {
                if let Ok(crate_edges) = load_cargo_dependencies(&manifest_path) {
                    // Extract crate name from path
                    if let Some(crate_name) = member.strip_prefix("crates/") {
                        crate_paths.insert(crate_name.to_string(), member.clone());
                    }
                    edges.extend(crate_edges);
                }
            }
        }

        Ok(Self { edges, crate_paths })
    }

    fn extract_workspace_members(toml_content: &str) -> Result<Vec<String>, String> {
        let mut members = Vec::new();

        // Find the members section
        let mut capturing = false;

        for line in toml_content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("[") {
                capturing = trimmed.starts_with("workspace.members");
                if !capturing && !members.is_empty() {
                    break;
                }
            }

            if capturing {
                if trimmed.starts_with('"') {
                    let member = trimmed.trim_matches('"').trim_matches(',').trim();
                    if !member.is_empty() {
                        members.push(member.to_string());
                    }
                }
            }
        }

        Ok(members)
    }

    /// Check for forbidden dependency edges
    pub fn check_violations(&self, checker: &DependencyChecker) -> Vec<DependencyViolation> {
        let mut violations = Vec::new();

        for (from, to) in &self.edges {
            if let (Some(from_crate), Some(to_crate)) =
                (CrateName::from_str(from), CrateName::from_str(to))
            {
                if let Some(violation) = checker.check_dependency(from_crate, to_crate) {
                    violations.push(violation);
                }
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_name_as_str() {
        assert_eq!(CrateName::Core.as_str(), "ai-history-core");
        assert_eq!(CrateName::Fsm.as_str(), "ai-history-fsm");
        assert_eq!(CrateName::UiModel.as_str(), "ai-history-ui-model");
    }

    #[test]
    fn test_crate_name_from_str() {
        assert_eq!(
            CrateName::from_str("ai-history-core"),
            Some(CrateName::Core)
        );
        assert_eq!(CrateName::from_str("ai-history-fsm"), Some(CrateName::Fsm));
        assert_eq!(CrateName::from_str("unknown"), None);
    }

    #[test]
    fn test_dependency_rules_core_store_forbidden() {
        let rules = DependencyRules::new();
        assert_eq!(
            rules.is_allowed(CrateName::Core, CrateName::Store),
            Some(&DependencyRule::Forbidden)
        );
    }

    #[test]
    fn test_dependency_rules_core_core_allowed() {
        let rules = DependencyRules::new();
        assert!(rules.is_allowed(CrateName::Cli, CrateName::Core).is_none());
    }

    #[test]
    fn test_dependency_checker_fsm_store_violation() {
        let checker = DependencyChecker::new();
        let violation = checker.check_dependency(CrateName::Fsm, CrateName::Store);
        assert!(violation.is_some());
        assert_eq!(violation.unwrap().from, CrateName::Fsm);
    }

    #[test]
    fn test_dependency_checker_core_restore_violation() {
        let checker = DependencyChecker::new();
        let violation = checker.check_dependency(CrateName::Core, CrateName::Restore);
        assert!(violation.is_some());
    }

    #[test]
    fn test_dependency_checker_ui_model_violation() {
        let checker = DependencyChecker::new();
        // UI model depending on store should be forbidden
        let violation = checker.check_dependency(CrateName::UiModel, CrateName::Store);
        assert!(violation.is_some());

        // UI model depending on cli should be forbidden
        let violation2 = checker.check_dependency(CrateName::UiModel, CrateName::Cli);
        assert!(violation2.is_some());
    }

    #[test]
    fn test_dependency_checker_allowed() {
        let checker = DependencyChecker::new();
        // These should be allowed
        let v1 = checker.check_dependency(CrateName::Cli, CrateName::Core);
        let v2 = checker.check_dependency(CrateName::Store, CrateName::Core);
        let v3 = checker.check_dependency(CrateName::Restore, CrateName::Snapshot);

        assert!(v1.is_none(), "Cli -> Core should be allowed");
        assert!(v2.is_none(), "Store -> Core should be allowed");
        assert!(v3.is_none(), "Restore -> Snapshot should be allowed");
    }

    #[test]
    fn test_dependency_checker_all() {
        let checker = DependencyChecker::new();
        let deps = vec![
            ("ai-history-fsm".to_string(), "ai-history-store".to_string()),
            ("ai-history-cli".to_string(), "ai-history-core".to_string()),
        ];

        let violations = checker.check_all(&deps);
        assert_eq!(violations.len(), 1); // Only FSM->Store is forbidden
        assert_eq!(violations[0].from, CrateName::Fsm);
    }

    #[test]
    fn test_dependency_checker_no_violations() {
        let checker = DependencyChecker::new();
        let deps = vec![
            ("ai-history-cli".to_string(), "ai-history-core".to_string()),
            (
                "ai-history-store".to_string(),
                "ai-history-core".to_string(),
            ),
        ];

        let violations = checker.check_all(&deps);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_find_cycles_no_cycles() {
        let checker = DependencyChecker::new();
        let edges = vec![
            ("a".to_string(), "b".to_string()),
            ("b".to_string(), "c".to_string()),
        ];

        let cycles = checker.find_cycles(&edges);
        assert!(cycles.is_empty());
    }

    #[test]
    fn test_find_cycles_with_cycle() {
        let checker = DependencyChecker::new();
        let edges = vec![
            ("a".to_string(), "b".to_string()),
            ("b".to_string(), "c".to_string()),
            ("c".to_string(), "a".to_string()),
        ];

        let cycles = checker.find_cycles(&edges);
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_dependency_rule_display() {
        let rule = DependencyRule::Forbidden;
        assert_eq!(format!("{:?}", rule), "Forbidden");
    }

    #[test]
    fn test_parse_dependency_line_simple() {
        let line = r#"ai-history-store = { path = "../ai-history-store" }"#;
        let dep = parse_dependency_line(line).unwrap();
        assert_eq!(dep.name, "ai-history-store");
        assert_eq!(dep.path, Some("../ai-history-store".to_string()));
        assert!(dep.version.is_none());
    }

    #[test]
    fn test_parse_dependency_line_version() {
        let line = r#"uuid = { version = "1.0", features = ["v4", "serde"] }"#;
        let dep = parse_dependency_line(line).unwrap();
        assert_eq!(dep.name, "uuid");
        assert_eq!(dep.version, Some("1.0".to_string()));
        assert!(dep.features.contains(&"v4".to_string()));
        assert!(dep.features.contains(&"serde".to_string()));
    }

    #[test]
    fn test_parse_dependency_line_optional() {
        let line = r#"ai-history-store = { path = "../ai-history-store", optional = "true" }"#;
        let dep = parse_dependency_line(line).unwrap();
        assert_eq!(dep.name, "ai-history-store");
        assert!(dep.optional);
    }

    #[test]
    fn test_parse_cargo_toml_dependencies_simple() {
        let toml = r#"
[dependencies]
ai-history-core = { path = "../ai-history-core" }
ai-history-events = { path = "../ai-history-events" }
thiserror = { workspace = true }
"#;
        let deps = parse_cargo_toml_dependencies(toml);
        // ai-history crates should be captured with their paths
        assert!(deps.iter().any(
            |d| d.name == "ai-history-core" && d.path == Some("../ai-history-core".to_string())
        ));
        assert!(deps
            .iter()
            .any(|d| d.name == "ai-history-events"
                && d.path == Some("../ai-history-events".to_string())));
        // thiserror with workspace=true has version "true"
        assert!(deps
            .iter()
            .any(|d| d.name == "thiserror" && d.version == Some("true".to_string())));
    }

    #[test]
    fn test_load_cargo_dependencies_invalid_path() {
        let result = load_cargo_dependencies(Path::new("/nonexistent/path/Cargo.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_cargo_dependency_debug() {
        let dep = CargoDependency {
            name: "ai-history-core".to_string(),
            version: None,
            path: Some("../ai-history-core".to_string()),
            features: vec!["feature1".to_string()],
            optional: false,
        };
        let debug = format!("{:?}", dep);
        assert!(debug.contains("ai-history-core"));
    }

    #[test]
    fn test_workspace_dependency_graph_empty() {
        let graph = WorkspaceDependencyGraph {
            edges: Vec::new(),
            crate_paths: HashMap::new(),
        };
        let checker = DependencyChecker::new();
        let violations = graph.check_violations(&checker);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_workspace_dependency_graph_violation() {
        let mut crate_paths = HashMap::new();
        crate_paths.insert(
            "ai-history-fsm".to_string(),
            "crates/ai-history-fsm".to_string(),
        );
        crate_paths.insert(
            "ai-history-store".to_string(),
            "crates/ai-history-store".to_string(),
        );

        let graph = WorkspaceDependencyGraph {
            edges: vec![("ai-history-fsm".to_string(), "ai-history-store".to_string())],
            crate_paths,
        };

        let checker = DependencyChecker::new();
        let violations = graph.check_violations(&checker);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].from, CrateName::Fsm);
        assert_eq!(violations[0].to, CrateName::Store);
    }

    #[test]
    fn test_workspace_dependency_graph_no_violation() {
        let mut crate_paths = HashMap::new();
        crate_paths.insert(
            "ai-history-cli".to_string(),
            "crates/ai-history-cli".to_string(),
        );
        crate_paths.insert(
            "ai-history-core".to_string(),
            "crates/ai-history-core".to_string(),
        );

        let graph = WorkspaceDependencyGraph {
            edges: vec![("ai-history-cli".to_string(), "ai-history-core".to_string())],
            crate_paths,
        };

        let checker = DependencyChecker::new();
        let violations = graph.check_violations(&checker);
        assert!(violations.is_empty());
    }
}
