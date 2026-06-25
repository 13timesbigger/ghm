use crate::error::{GhmError, Result};
use crate::github::client::GithubClient;
use crate::models::GithubProject;
use serde::Deserialize;

/// GraphQL query to list Projects v2 for a user/viewer.
const VIEWER_PROJECTS_QUERY: &str = r#"
query($first: Int!) {
  viewer {
    projectsV2(first: $first) {
      nodes {
        id
        title
        number
        shortDescription
        closed
        url
      }
    }
  }
}
"#;

/// GraphQL query to list Projects v2 for an organisation.
const ORG_PROJECTS_QUERY: &str = r#"
query($org: String!, $first: Int!) {
  organization(login: $org) {
    projectsV2(first: $first) {
      nodes {
        id
        title
        number
        shortDescription
        closed
        url
      }
    }
  }
}
"#;

// ── Response shapes ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct ViewerData {
    viewer: ViewerProjects,
}

#[derive(Debug, Deserialize)]
struct ViewerProjects {
    #[serde(rename = "projectsV2")]
    projects_v2: ProjectNodes,
}

#[derive(Debug, Deserialize)]
struct OrgData {
    organization: OrgProjects,
}

#[derive(Debug, Deserialize)]
struct OrgProjects {
    #[serde(rename = "projectsV2")]
    projects_v2: ProjectNodes,
}

#[derive(Debug, Deserialize)]
struct ProjectNodes {
    nodes: Vec<ProjectNode>,
}

#[derive(Debug, Deserialize)]
struct ProjectNode {
    id: String,
    title: String,
    number: u64,
    #[serde(rename = "shortDescription")]
    short_description: Option<String>,
    closed: bool,
    url: String,
}

impl From<ProjectNode> for GithubProject {
    fn from(n: ProjectNode) -> Self {
        GithubProject {
            id: n.id,
            title: n.title,
            number: n.number,
            short_description: n.short_description,
            closed: n.closed,
            url: n.url,
        }
    }
}

// ── Public API ─────────────────────────────────────────────────────

/// List GitHub Projects v2 for the authenticated user.
pub async fn list_projects(client: &GithubClient) -> Result<Vec<GithubProject>> {
    let vars = serde_json::json!({ "first": 50 });
    let resp: GraphQLResponse<ViewerData> = client.graphql(VIEWER_PROJECTS_QUERY, vars).await?;

    if let Some(errors) = resp.errors {
        let msgs: Vec<_> = errors.iter().map(|e| e.message.as_str()).collect();
        return Err(GhmError::GraphQL {
            message: msgs.join("; "),
        });
    }

    let data = resp.data.ok_or_else(|| GhmError::GraphQL {
        message: "no data in GraphQL response".into(),
    })?;

    Ok(data
        .viewer
        .projects_v2
        .nodes
        .into_iter()
        .map(GithubProject::from)
        .collect())
}

/// List GitHub Projects v2 for an organisation.
pub async fn list_projects_by_org(
    client: &GithubClient,
    org: &str,
) -> Result<Vec<GithubProject>> {
    let vars = serde_json::json!({ "org": org, "first": 50 });
    let resp: GraphQLResponse<OrgData> = client.graphql(ORG_PROJECTS_QUERY, vars).await?;

    if let Some(errors) = resp.errors {
        let msgs: Vec<_> = errors.iter().map(|e| e.message.as_str()).collect();
        return Err(GhmError::GraphQL {
            message: msgs.join("; "),
        });
    }

    let data = resp.data.ok_or_else(|| GhmError::GraphQL {
        message: "no data in GraphQL response".into(),
    })?;

    Ok(data
        .organization
        .projects_v2
        .nodes
        .into_iter()
        .map(GithubProject::from)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_node_to_github_project() {
        let node = ProjectNode {
            id: "PVT_abc".into(),
            title: "My Project".into(),
            number: 1,
            short_description: Some("A project".into()),
            closed: false,
            url: "https://github.com/orgs/o/projects/1".into(),
        };
        let project = GithubProject::from(node);
        assert_eq!(project.id, "PVT_abc");
        assert_eq!(project.title, "My Project");
        assert_eq!(project.number, 1);
        assert!(!project.closed);
    }

    #[test]
    fn project_node_to_github_project_closed() {
        let node = ProjectNode {
            id: "PVT_xyz".into(),
            title: "Done".into(),
            number: 5,
            short_description: None,
            closed: true,
            url: "https://github.com/orgs/o/projects/5".into(),
        };
        let project = GithubProject::from(node);
        assert!(project.closed);
        assert!(project.short_description.is_none());
    }

    #[test]
    fn graphql_response_with_errors() {
        let json = r#"{
            "data": null,
            "errors": [{"message": "Not Found"}]
        }"#;
        let resp: GraphQLResponse<ViewerData> = serde_json::from_str(json).unwrap();
        assert!(resp.data.is_none());
        assert_eq!(resp.errors.unwrap()[0].message, "Not Found");
    }

    #[test]
    fn graphql_response_with_data() {
        let json = r#"{
            "data": {
                "viewer": {
                    "projectsV2": {
                        "nodes": [
                            {
                                "id": "PVT_1",
                                "title": "Test",
                                "number": 1,
                                "shortDescription": null,
                                "closed": false,
                                "url": "https://github.com/users/u/projects/1"
                            }
                        ]
                    }
                }
            }
        }"#;
        let resp: GraphQLResponse<ViewerData> = serde_json::from_str(json).unwrap();
        let data = resp.data.unwrap();
        assert_eq!(data.viewer.projects_v2.nodes.len(), 1);
        assert_eq!(data.viewer.projects_v2.nodes[0].title, "Test");
    }

    #[test]
    fn org_graphql_response() {
        let json = r#"{
            "data": {
                "organization": {
                    "projectsV2": {
                        "nodes": []
                    }
                }
            }
        }"#;
        let resp: GraphQLResponse<OrgData> = serde_json::from_str(json).unwrap();
        let data = resp.data.unwrap();
        assert!(data.organization.projects_v2.nodes.is_empty());
    }

    #[test]
    fn queries_are_non_empty() {
        assert!(!VIEWER_PROJECTS_QUERY.is_empty());
        assert!(!ORG_PROJECTS_QUERY.is_empty());
        assert!(VIEWER_PROJECTS_QUERY.contains("projectsV2"));
        assert!(ORG_PROJECTS_QUERY.contains("organization"));
    }

    #[test]
    fn github_project_serde_roundtrip() {
        let project = GithubProject {
            id: "PVT_1".into(),
            title: "Proj".into(),
            number: 3,
            short_description: Some("desc".into()),
            closed: false,
            url: "https://example.com".into(),
        };
        let json = serde_json::to_string(&project).unwrap();
        let back: GithubProject = serde_json::from_str(&json).unwrap();
        assert_eq!(back, project);
    }
}
