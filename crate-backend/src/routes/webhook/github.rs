use std::default::Default;
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use common::v1::types::{EmbedCreate, MessageCreate, WebhookId};
use serde::Deserialize;
use tracing::debug;

use crate::{
    error::{Error, Result},
    ServerState,
};

#[derive(Debug, Deserialize)]
struct GitHubUser {
    name: String,
    login: String,
}

#[derive(Debug, Deserialize)]
struct GitHubCommit {
    id: String,
    message: String,
    url: String,
    author: GitHubUser,
}

#[derive(Debug, Deserialize)]
struct GitHubRepository {
    name: String,
    full_name: String,
    html_url: String,
}

#[derive(Debug, Deserialize)]
struct PushEvent {
    #[serde(rename = "ref")]
    reference: String,
    repository: GitHubRepository,
    pusher: GitHubUser,
    commits: Vec<GitHubCommit>,
    compare: String,
}

#[derive(Debug, Deserialize)]
struct Issue {
    html_url: String,
    number: u64,
    title: String,
    user: GitHubUser,
}

#[derive(Debug, Deserialize)]
struct IssuesEvent {
    action: String,
    issue: Issue,
    repository: GitHubRepository,
    sender: GitHubUser,
}

#[derive(Debug, Deserialize)]
struct PullRequest {
    html_url: String,
    number: u64,
    title: String,
    user: GitHubUser,
}

#[derive(Debug, Deserialize)]
struct PullRequestEvent {
    action: String,
    pull_request: PullRequest,
    repository: GitHubRepository,
    sender: GitHubUser,
}

fn handle_push_event(event: PushEvent) -> Result<MessageCreate> {
    let mut description = String::new();
    for commit in &event.commits {
        let short_id = &commit.id[..7];
        description.push_str(&format!(
            "[`{}`]({})
 {} - {}",
            short_id,
            commit.url,
            commit.message.lines().next().unwrap_or(""),
            commit.author.name
        ));
    }

    let branch = event.reference.replace("refs/heads/", "");

    let embed = EmbedCreate {
        title: Some(format!(
            "[{}:{}] {} new commits",
            event.repository.name,
            branch,
            event.commits.len()
        )),
        description: Some(description),
        url: Some(event.compare.parse()?),
        author_name: Some(event.pusher.name.clone()),
        ..Default::default()
    };

    Ok(MessageCreate {
        embeds: vec![embed],
        ..Default::default()
    })
}

fn handle_issues_event(event: IssuesEvent) -> Result<MessageCreate> {
    let action = event.action;
    let color = match action.as_str() {
        "opened" | "reopened" => Some("#28a745"), // green
        "closed" => Some("#d73a49"),              // red
        _ => Some("#6a737d"),                     // gray
    };

    let embed = EmbedCreate {
        title: Some(format!(
            "[{}] Issue #{}: {}",
            event.repository.full_name, event.issue.number, event.issue.title
        )),
        description: Some(format!("Issue {} by {}", action, event.issue.user.login)),
        url: Some(event.issue.html_url.parse()?),
        color: color.map(String::from),
        author_name: Some(event.sender.login.clone()),
        ..Default::default()
    };

    Ok(MessageCreate {
        embeds: vec![embed],
        ..Default::default()
    })
}

fn handle_pull_request_event(event: PullRequestEvent) -> Result<MessageCreate> {
    let action = event.action;
    let color = match action.as_str() {
        "opened" | "reopened" => Some("#28a745"), // green
        "closed" => Some("#d73a49"),              // red
        _ => Some("#6a737d"),                     // gray
    };

    let embed = EmbedCreate {
        title: Some(format!(
            "[{}] Pull Request #{}: {}",
            event.repository.full_name, event.pull_request.number, event.pull_request.title
        )),
        description: Some(format!("Pull Request {} by {}", action, event.sender.login)),
        url: Some(event.pull_request.html_url.parse()?),
        color: color.map(String::from),
        author_name: Some(event.sender.login.clone()),
        ..Default::default()
    };

    Ok(MessageCreate {
        embeds: vec![embed],
        ..Default::default()
    })
}

/// Webhook execute github (WIP)
#[utoipa::path(
    post,
    path = "/webhook/{webhook_id}/{token}/github",
    params(
        ("webhook_id", description = "Webhook id"),
        ("token", description = "Webhook token")
    ),
    tags = ["webhook"],
    responses((
        status = NO_CONTENT,
        description = "Execute webhook success",
    ))
)]
pub async fn webhook_execute_github(
    Path((webhook_id, token)): Path<(WebhookId, String)>,
    State(s): State<Arc<ServerState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse> {
    let webhook = s.data().webhook_get_with_token(webhook_id, &token).await?;

    let event_type = headers
        .get("X-GitHub-Event")
        .ok_or(Error::BadRequest(
            "Missing X-GitHub-Event header".to_string(),
        ))?
        .to_str()?;

    debug!("received github webhook event: {}", event_type);

    let message_create = match event_type {
        "push" => {
            let event: PushEvent = serde_json::from_value(body)?;
            handle_push_event(event)?
        }
        "issues" => {
            let event: IssuesEvent = serde_json::from_value(body)?;
            handle_issues_event(event)?
        }
        "pull_request" => {
            let event: PullRequestEvent = serde_json::from_value(body)?;
            handle_pull_request_event(event)?
        }
        _ => {
            return Ok(StatusCode::NO_CONTENT);
        }
    };

    let author_id = (*webhook.id).into();
    let channel_id = webhook.channel_id;

    let srv = s.services();
    srv.messages
        .create(channel_id, author_id, None, None, message_create)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
