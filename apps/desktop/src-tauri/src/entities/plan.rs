use serde::{Deserialize, Serialize};

/// A task entry in the PLAN.yaml file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTask {
    /// Unique identifier for this task within the plan
    pub id: String,
    /// Human-readable title for this task (defaults to id if not specified)
    #[serde(default)]
    pub title: Option<String>,
    /// Task description for the AI agent
    pub prompt: String,
    /// Custom branch name for this task (uses template if not specified)
    #[serde(default, rename = "branchName")]
    pub branch_name: Option<String>,
    /// List of task IDs that must complete before this task starts
    #[serde(default, rename = "dependsOn")]
    pub depends_on: Vec<String>,
}

/// The structure of a PLAN.yaml file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanYaml {
    /// List of tasks in the plan
    pub tasks: Vec<PlanTask>,
}

impl PlanYaml {
    /// Parses a PLAN.yaml file content into a PlanYaml structure
    pub fn parse(content: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(content)
    }

    /// Validates the plan structure
    pub fn validate(&self) -> Result<(), PlanValidationError> {
        // Check for empty tasks
        if self.tasks.is_empty() {
            return Err(PlanValidationError::EmptyPlan);
        }

        // Collect all task IDs
        let task_ids: Vec<&str> = self.tasks.iter().map(|t| t.id.as_str()).collect();

        // Check for unique IDs
        let mut seen_ids = std::collections::HashSet::new();
        for id in &task_ids {
            if !seen_ids.insert(*id) {
                return Err(PlanValidationError::DuplicateId(id.to_string()));
            }
        }

        // Check for valid references in dependsOn
        for task in &self.tasks {
            for dep in &task.depends_on {
                if !task_ids.contains(&dep.as_str()) {
                    return Err(PlanValidationError::InvalidDependency {
                        task_id: task.id.clone(),
                        dependency_id: dep.clone(),
                    });
                }
            }
        }

        // Check for non-empty prompts
        for task in &self.tasks {
            if task.prompt.trim().is_empty() {
                return Err(PlanValidationError::EmptyPrompt(task.id.clone()));
            }
        }

        // Check for cycles using DFS
        if self.has_cycle() {
            return Err(PlanValidationError::CyclicDependency);
        }

        Ok(())
    }

    /// Checks if the dependency graph has a cycle using DFS
    fn has_cycle(&self) -> bool {
        use std::collections::HashMap;

        // Build adjacency list
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for task in &self.tasks {
            adj.insert(
                &task.id,
                task.depends_on.iter().map(|s| s.as_str()).collect(),
            );
        }

        #[derive(Clone, Copy, PartialEq)]
        enum State {
            Unvisited,
            Visiting,
            Visited,
        }

        let mut state: HashMap<&str, State> = HashMap::new();
        for task in &self.tasks {
            state.insert(&task.id, State::Unvisited);
        }

        fn dfs<'a>(
            node: &'a str,
            adj: &HashMap<&'a str, Vec<&'a str>>,
            state: &mut HashMap<&'a str, State>,
        ) -> bool {
            match state.get(node) {
                Some(State::Visiting) => return true, // Cycle detected
                Some(State::Visited) => return false, // Already processed
                _ => {}
            }

            state.insert(node, State::Visiting);

            if let Some(neighbors) = adj.get(node) {
                for neighbor in neighbors {
                    if dfs(neighbor, adj, state) {
                        return true;
                    }
                }
            }

            state.insert(node, State::Visited);
            false
        }

        for task in &self.tasks {
            if state.get(task.id.as_str()) == Some(&State::Unvisited)
                && dfs(&task.id, &adj, &mut state)
            {
                return true;
            }
        }

        false
    }
}

/// Errors that can occur during plan validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanValidationError {
    /// Plan has no tasks
    EmptyPlan,
    /// Duplicate task ID found
    DuplicateId(String),
    /// A task references a non-existent dependency
    InvalidDependency {
        task_id: String,
        dependency_id: String,
    },
    /// A task has an empty prompt
    EmptyPrompt(String),
    /// The dependency graph contains a cycle
    CyclicDependency,
}

impl std::fmt::Display for PlanValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPlan => write!(f, "Plan contains no tasks"),
            Self::DuplicateId(id) => write!(f, "Duplicate task ID: {}", id),
            Self::InvalidDependency {
                task_id,
                dependency_id,
            } => write!(
                f,
                "Task '{}' depends on non-existent task '{}'",
                task_id, dependency_id
            ),
            Self::EmptyPrompt(id) => write!(f, "Task '{}' has an empty prompt", id),
            Self::CyclicDependency => write!(f, "Plan contains a cyclic dependency"),
        }
    }
}

impl std::error::Error for PlanValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_plan() {
        let content = r#"
tasks:
  - id: "setup-db"
    prompt: "Set up database schema"
  - id: "api"
    prompt: "Implement API endpoints"
    dependsOn: ["setup-db"]
"#;

        let plan = PlanYaml::parse(content).unwrap();
        assert_eq!(plan.tasks.len(), 2);
        assert_eq!(plan.tasks[0].id, "setup-db");
        assert_eq!(plan.tasks[1].depends_on, vec!["setup-db"]);
    }

    #[test]
    fn test_parse_plan_with_title_and_branch_name() {
        let content = r#"
tasks:
  - id: "setup-db"
    title: "Setup Database"
    prompt: "Set up database schema"
    branchName: "feature/setup-db"
  - id: "api"
    title: "Implement API"
    prompt: "Implement API endpoints"
    branchName: "feature/api-endpoints"
    dependsOn: ["setup-db"]
"#;

        let plan = PlanYaml::parse(content).unwrap();
        assert_eq!(plan.tasks.len(), 2);
        assert_eq!(plan.tasks[0].id, "setup-db");
        assert_eq!(plan.tasks[0].title, Some("Setup Database".to_string()));
        assert_eq!(
            plan.tasks[0].branch_name,
            Some("feature/setup-db".to_string())
        );
        assert_eq!(plan.tasks[1].title, Some("Implement API".to_string()));
        assert_eq!(
            plan.tasks[1].branch_name,
            Some("feature/api-endpoints".to_string())
        );
    }

    #[test]
    fn test_parse_plan_without_optional_fields() {
        let content = r#"
tasks:
  - id: "task1"
    prompt: "Do something"
"#;

        let plan = PlanYaml::parse(content).unwrap();
        assert_eq!(plan.tasks.len(), 1);
        assert_eq!(plan.tasks[0].id, "task1");
        assert_eq!(plan.tasks[0].title, None);
        assert_eq!(plan.tasks[0].branch_name, None);
        assert!(plan.tasks[0].depends_on.is_empty());
    }

    #[test]
    fn test_validate_empty_plan() {
        let plan = PlanYaml { tasks: vec![] };
        assert_eq!(plan.validate(), Err(PlanValidationError::EmptyPlan));
    }

    #[test]
    fn test_validate_duplicate_id() {
        let plan = PlanYaml {
            tasks: vec![
                PlanTask {
                    id: "task1".to_string(),
                    title: None,
                    prompt: "Do something".to_string(),
                    branch_name: None,
                    depends_on: vec![],
                },
                PlanTask {
                    id: "task1".to_string(),
                    title: None,
                    prompt: "Do something else".to_string(),
                    branch_name: None,
                    depends_on: vec![],
                },
            ],
        };
        assert_eq!(
            plan.validate(),
            Err(PlanValidationError::DuplicateId("task1".to_string()))
        );
    }

    #[test]
    fn test_validate_invalid_dependency() {
        let plan = PlanYaml {
            tasks: vec![PlanTask {
                id: "task1".to_string(),
                title: None,
                prompt: "Do something".to_string(),
                branch_name: None,
                depends_on: vec!["nonexistent".to_string()],
            }],
        };
        assert_eq!(
            plan.validate(),
            Err(PlanValidationError::InvalidDependency {
                task_id: "task1".to_string(),
                dependency_id: "nonexistent".to_string(),
            })
        );
    }

    #[test]
    fn test_validate_empty_prompt() {
        let plan = PlanYaml {
            tasks: vec![PlanTask {
                id: "task1".to_string(),
                title: None,
                prompt: "   ".to_string(),
                branch_name: None,
                depends_on: vec![],
            }],
        };
        assert_eq!(
            plan.validate(),
            Err(PlanValidationError::EmptyPrompt("task1".to_string()))
        );
    }

    #[test]
    fn test_validate_cyclic_dependency() {
        let plan = PlanYaml {
            tasks: vec![
                PlanTask {
                    id: "task1".to_string(),
                    title: None,
                    prompt: "Task 1".to_string(),
                    branch_name: None,
                    depends_on: vec!["task2".to_string()],
                },
                PlanTask {
                    id: "task2".to_string(),
                    title: None,
                    prompt: "Task 2".to_string(),
                    branch_name: None,
                    depends_on: vec!["task1".to_string()],
                },
            ],
        };
        assert_eq!(plan.validate(), Err(PlanValidationError::CyclicDependency));
    }

    #[test]
    fn test_validate_valid_plan() {
        let plan = PlanYaml {
            tasks: vec![
                PlanTask {
                    id: "setup".to_string(),
                    title: Some("Project Setup".to_string()),
                    prompt: "Set up the project".to_string(),
                    branch_name: Some("feature/setup".to_string()),
                    depends_on: vec![],
                },
                PlanTask {
                    id: "implement".to_string(),
                    title: Some("Implement Feature".to_string()),
                    prompt: "Implement the feature".to_string(),
                    branch_name: None,
                    depends_on: vec!["setup".to_string()],
                },
                PlanTask {
                    id: "test".to_string(),
                    title: None,
                    prompt: "Write tests".to_string(),
                    branch_name: Some("feature/tests".to_string()),
                    depends_on: vec!["implement".to_string()],
                },
            ],
        };
        assert!(plan.validate().is_ok());
    }
}
