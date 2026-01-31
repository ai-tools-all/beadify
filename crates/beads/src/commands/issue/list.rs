//! Issue list command with natural CLI interface
//!
//! Reuses existing list logic but adds filtering by kind and priority.

use anyhow::Result;
use beads_core::{
    get_all_issues, get_issue, get_issue_labels, get_open_dependencies, repo::BeadsRepo,
};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

fn status_indicator(status: &str) -> &'static str {
    match status {
        "closed" => "●",
        _ => "☐",
    }
}

fn parse_labels(label_str: &str) -> Vec<String> {
    label_str.split(',').map(|s| s.trim().to_string()).collect()
}

fn issue_has_all_labels(
    repo: &BeadsRepo,
    issue_id: &str,
    required_labels: &[String],
) -> Result<bool> {
    let issue_labels = get_issue_labels(repo, issue_id)?;
    let issue_label_names: Vec<String> = issue_labels.iter().map(|l| l.name.clone()).collect();
    Ok(required_labels
        .iter()
        .all(|label| issue_label_names.contains(label)))
}

fn issue_has_any_label(
    repo: &BeadsRepo,
    issue_id: &str,
    required_labels: &[String],
) -> Result<bool> {
    let issue_labels = get_issue_labels(repo, issue_id)?;
    let issue_label_names: Vec<String> = issue_labels.iter().map(|l| l.name.clone()).collect();
    Ok(required_labels
        .iter()
        .any(|label| issue_label_names.contains(label)))
}

struct TreeNode {
    issue: beads_core::Issue,
    children: Vec<String>,
}

fn build_dependency_graph(
    repo: &BeadsRepo,
    issues: &[beads_core::Issue],
) -> Result<HashMap<String, TreeNode>> {
    let mut graph: HashMap<String, TreeNode> = HashMap::new();

    // Initialize all nodes
    for issue in issues {
        graph.insert(
            issue.id.clone(),
            TreeNode {
                issue: issue.clone(),
                children: Vec::new(),
            },
        );
    }

    // Build parent-child relationships
    for issue in issues {
        let deps = get_open_dependencies(repo, &issue.id)?;
        for dep_id in deps {
            if let Some(parent_node) = graph.get_mut(&dep_id) {
                parent_node.children.push(issue.id.clone());
            }
        }
    }

    Ok(graph)
}

fn find_roots(graph: &HashMap<String, TreeNode>, _repo: &BeadsRepo) -> Result<Vec<String>> {
    let mut is_dependency: HashSet<String> = HashSet::new();

    // Mark all issues that are dependencies of others
    for node in graph.values() {
        for child_id in &node.children {
            is_dependency.insert(child_id.clone());
        }
    }

    // Roots are issues that are not dependencies of any other issue
    let mut roots: Vec<String> = graph
        .keys()
        .filter(|id| !is_dependency.contains(*id))
        .cloned()
        .collect();

    // Sort roots by priority (descending) and then by id
    roots.sort_by(|a, b| {
        let node_a = &graph[a];
        let node_b = &graph[b];
        node_b
            .issue
            .priority
            .cmp(&node_a.issue.priority)
            .then_with(|| a.cmp(b))
    });

    Ok(roots)
}

fn print_tree_node(
    repo: &BeadsRepo,
    graph: &HashMap<String, TreeNode>,
    node_id: &str,
    prefix: &str,
    depth: usize,
    is_last: bool,
    show_labels: bool,
) -> Result<()> {
    let node = &graph[node_id];
    let issue = &node.issue;

    let indicator = status_indicator(&issue.status);
    let priority_str = format!("p{}", issue.priority);

    // Get labels if needed
    let labels_str = if show_labels {
        match get_issue_labels(repo, &issue.id) {
            Ok(labels) => {
                let label_names: Vec<String> = labels.iter().map(|l| l.name.clone()).collect();
                if label_names.is_empty() {
                    "-".to_string()
                } else {
                    format!(" [{}]", label_names.join(", "))
                }
            }
            Err(_) => "-".to_string(),
        }
    } else {
        String::new()
    };

    // Print current node
    if depth == 0 {
        println!(
            "{} {:<8} {:<10} {:<4}{}{}",
            indicator, issue.id, issue.kind, priority_str, labels_str, issue.title
        );
    } else {
        let branch = if is_last { "└─ " } else { "├─ " };
        println!(
            "{}{}{} {:<8} {:<10} {:<4}{}{}",
            prefix, branch, indicator, issue.id, issue.kind, priority_str, labels_str, issue.title
        );
    }

    // Prepare for children
    let child_count = node.children.len();
    if child_count > 0 {
        // Sort children by priority (descending) and then by id
        let mut sorted_children = node.children.clone();
        sorted_children.sort_by(|a, b| {
            let node_a = &graph[a];
            let node_b = &graph[b];
            node_b
                .issue
                .priority
                .cmp(&node_a.issue.priority)
                .then_with(|| a.cmp(b))
        });

        let new_prefix = if depth == 0 {
            String::new()
        } else {
            format!("{}{}", prefix, if is_last { "   " } else { "│  " })
        };

        for (idx, child_id) in sorted_children.iter().enumerate() {
            let is_last_child = idx == child_count - 1;
            print_tree_node(
                repo,
                graph,
                child_id,
                &new_prefix,
                depth + 1,
                is_last_child,
                show_labels,
            )?;
        }
    }

    Ok(())
}

/// Parameters for listing issues
pub struct ListParams {
    pub show_all: bool,
    pub status_filter: Option<String>,
    pub priority_filter: Option<u32>,
    pub kind_filter: Option<String>,
    pub label_filter: Option<String>,
    pub label_any_filter: Option<String>,
    pub flat: bool,
    pub json_output: bool,
    pub show_labels: bool,
    pub created_after: Option<String>,
    pub created_before: Option<String>,
    pub timezone: Option<String>,
}

/// Run the issue list command
pub fn run(repo: BeadsRepo, params: ListParams) -> Result<()> {
    let ListParams {
        show_all,
        status_filter,
        priority_filter,
        kind_filter,
        label_filter,
        label_any_filter,
        flat,
        json_output,
        show_labels,
        created_after,
        created_before,
        timezone,
    } = params;
    // Get user's timezone for date parsing
    let user_tz = beads_core::tz::get_user_timezone(timezone.as_deref())?;

    let mut issues = get_all_issues(&repo)?;

    // Filter issues based on status
    if let Some(status) = status_filter {
        issues.retain(|issue| issue.status == status);
    } else if !show_all {
        issues.retain(|issue| issue.status == "open");
    }

    // Filter by priority (pre-validated by clap as u32)
    if let Some(priority_int) = priority_filter {
        issues.retain(|issue| issue.priority == priority_int);
    }

    // Filter by kind
    if let Some(k) = kind_filter {
        let kind_lower = k.to_lowercase();
        issues.retain(|issue| issue.kind.to_lowercase() == kind_lower);
    }

    // Filter issues by labels (AND - must have ALL labels)
    if let Some(label_str) = label_filter {
        let required_labels = parse_labels(&label_str);
        issues.retain(|issue| {
            issue_has_all_labels(&repo, &issue.id, &required_labels).unwrap_or(false)
        });
    }

    // Filter issues by labels (OR - must have AT LEAST ONE label)
    if let Some(label_str) = label_any_filter {
        let required_labels = parse_labels(&label_str);
        issues.retain(|issue| {
            issue_has_any_label(&repo, &issue.id, &required_labels).unwrap_or(false)
        });
    }

    // Apply created_after filter
    if let Some(ref date_str) = created_after {
        let after_date = beads_core::query::parse_date(date_str, user_tz)?;
        issues.retain(|issue| beads_core::query::created_after(&issue.created_at, &after_date));
    }

    // Apply created_before filter
    if let Some(ref date_str) = created_before {
        let before_date = beads_core::query::parse_date(date_str, user_tz)?;
        issues.retain(|issue| beads_core::query::created_before(&issue.created_at, &before_date));
    }

    if issues.is_empty() {
        if json_output {
            println!("{}", serde_json::to_string_pretty(&json!({"issues": []}))?);
        } else {
            println!("No issues found.");
        }
        return Ok(());
    }

    if json_output {
        let issues_json: Vec<Value> = issues
            .iter()
            .map(|issue| {
                // Convert created_at to local timezone for display
                let created_at_local =
                    beads_core::tz::utc_to_local_string(&issue.created_at, user_tz)
                        .unwrap_or_else(|_| issue.created_at.clone());

                json!({
                    "id": issue.id,
                    "title": issue.title,
                    "kind": issue.kind,
                    "priority": issue.priority,
                    "status": issue.status,
                    "created_at_utc": issue.created_at,
                    "created_at_local": created_at_local,
                    "description": issue.description,
                    "design": issue.design,
                    "acceptance_criteria": issue.acceptance_criteria,
                    "notes": issue.notes,
                    "data": issue.data,
                    "labels": match get_issue_labels(&repo, &issue.id) {
                        Ok(labels) => labels.iter().map(|l| l.name.clone()).collect::<Vec<_>>(),
                        Err(_) => vec![],
                    }
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({"issues": issues_json}))?
        );
        return Ok(());
    }

    if !flat {
        // Tree view (default)
        // Build dependency graph
        let graph = build_dependency_graph(&repo, &issues)?;
        let roots = find_roots(&graph, &repo)?;

        if show_labels {
            println!(
                "{:<2} {:<8} {:<10} {:<4} {:<20} Title",
                " ", "ID", "Kind", "Prio", "Labels"
            );
            println!("{}", "─".repeat(100));
        } else {
            println!("{:<2} {:<8} {:<10} {:<4} Title", " ", "ID", "Kind", "Prio");
            println!("{}", "─".repeat(70));
        }

        for root_id in roots {
            print_tree_node(&repo, &graph, &root_id, "", 0, true, show_labels)?;
        }
    } else {
        // Flat list view (old behavior)
        if show_labels {
            println!(
                "{:<2} {:<8} {:<10} {:<4} {:<20} Title",
                " ", "ID", "Kind", "Prio", "Labels"
            );
            println!("{}", "─".repeat(100));

            for issue in issues {
                let indicator = status_indicator(&issue.status);
                let priority_str = format!("p{}", issue.priority);

                // Get labels for this issue
                let labels_str = match get_issue_labels(&repo, &issue.id) {
                    Ok(labels) => {
                        let label_names: Vec<String> =
                            labels.iter().map(|l| l.name.clone()).collect();
                        if label_names.is_empty() {
                            "-".to_string()
                        } else {
                            label_names.join(", ")
                        }
                    }
                    Err(_) => "-".to_string(),
                };

                println!(
                    "{} {:<8} {:<10} {:<4} {:<20} {}",
                    indicator, issue.id, issue.kind, priority_str, labels_str, issue.title
                );

                // Show open dependencies/blockers if any
                if let Ok(deps) = get_open_dependencies(&repo, &issue.id) {
                    for dep_id in deps {
                        if let Ok(Some(dep_issue)) = get_issue(&repo, &dep_id) {
                            let dep_priority = format!("p{}", dep_issue.priority);
                            println!(
                                "    ↳ {:<8} {:<10} {} - {}",
                                dep_id, dep_issue.kind, dep_priority, dep_issue.title
                            );
                        }
                    }
                }
            }
        } else {
            println!("{:<2} {:<8} {:<10} {:<4} Title", " ", "ID", "Kind", "Prio");
            println!("{}", "─".repeat(70));

            for issue in issues {
                let indicator = status_indicator(&issue.status);
                let priority_str = format!("p{}", issue.priority);

                println!(
                    "{} {:<8} {:<10} {:<4} {}",
                    indicator, issue.id, issue.kind, priority_str, issue.title
                );

                // Show open dependencies/blockers if any
                if let Ok(deps) = get_open_dependencies(&repo, &issue.id) {
                    for dep_id in deps {
                        if let Ok(Some(dep_issue)) = get_issue(&repo, &dep_id) {
                            let dep_priority = format!("p{}", dep_issue.priority);
                            println!(
                                "    ↳ {:<8} {:<10} {} - {}",
                                dep_id, dep_issue.kind, dep_priority, dep_issue.title
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
