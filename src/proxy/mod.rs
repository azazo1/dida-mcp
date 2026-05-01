pub mod types;

use std::{cmp::Ordering, collections::BTreeSet, error::Error, sync::Arc};

use axum::http::request::Parts;
use chrono::{Duration, Offset, Utc};
use chrono_tz::Tz;
use rmcp::{
    Json as McpJson, ServerHandler, ServiceExt,
    handler::server::wrapper::Parameters,
    model::{CallToolRequestParams, CallToolResult},
    service::{RequestContext, RoleServer},
    tool, tool_handler, tool_router,
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tracing::{error, warn};

use crate::{
    auth::extract_bearer_token,
    config::{AppConfig, RemoteBearerMode, normalized_token},
};

use self::types::{
    CompleteTaskArgs, CreateTaskArgs, CurrentTimeResult, GetCurrentTimeArgs,
    ListUndoneTasksByDateArgs, ProjectIdArgs, ProjectListResult, ProjectProfile, RemoteTaskPayload,
    RemoteUndoneTaskSearch, SearchTaskArgs, TaskIdArgs, UpdateTaskArgs, map_to_object,
};

#[derive(Clone)]
pub struct DidaProxy {
    config: Arc<AppConfig>,
}

fn format_error_chain(err: &(dyn Error + 'static)) -> String {
    let mut parts = Vec::new();
    let mut current = Some(err);

    while let Some(err) = current {
        let message = err.to_string();
        let should_push = parts
            .last()
            .map(|previous| previous != &message)
            .unwrap_or(true);

        if should_push {
            parts.push(message);
        }

        current = err.source();
    }

    parts.join(" | caused by: ")
}

fn tool_result_detail(result: &CallToolResult) -> String {
    if let Some(value) = result.structured_content.as_ref() {
        return value.to_string();
    }

    let text_parts = result
        .content
        .iter()
        .filter_map(|content| content.as_text().map(|text| text.text.clone()))
        .collect::<Vec<_>>();

    if text_parts.is_empty() {
        "remote tool returned an error result without details".to_owned()
    } else {
        text_parts.join(" | ")
    }
}

fn remote_tool_error(name: &str, result: &CallToolResult) -> String {
    format!(
        "remote tool `{name}` returned an error result: {}",
        tool_result_detail(result)
    )
}

fn collect_project_ids(value: &Value, project_ids: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, nested) in map {
                if matches!(key.as_str(), "projectId" | "project_id")
                    && let Some(project_id) = nested.as_str().map(str::trim)
                    && !project_id.is_empty()
                {
                    project_ids.insert(project_id.to_owned());
                }

                collect_project_ids(nested, project_ids);
            }
        }
        Value::Array(values) => {
            for nested in values {
                collect_project_ids(nested, project_ids);
            }
        }
        _ => {}
    }
}

fn sort_project_profiles(projects: &mut [ProjectProfile]) {
    projects.sort_by(|left, right| {
        match (left.sort_order, right.sort_order) {
            (Some(left), Some(right)) => left.cmp(&right),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
        .then_with(|| left.name.as_deref().cmp(&right.name.as_deref()))
        .then_with(|| left.id.as_deref().cmp(&right.id.as_deref()))
    });
}

impl DidaProxy {
    pub fn new(config: Arc<AppConfig>) -> Self {
        Self { config }
    }

    async fn call_remote_tool(
        &self,
        name: &str,
        arguments: serde_json::Map<String, serde_json::Value>,
        incoming_bearer_token: Option<&str>,
    ) -> Result<CallToolResult, String> {
        let remote_url = self.config.remote.url.clone();
        let mut transport_config =
            StreamableHttpClientTransportConfig::with_uri(remote_url.clone());

        if let Some(bearer) = self.resolve_remote_bearer_token(incoming_bearer_token)? {
            transport_config = transport_config.auth_header(bearer);
        }

        let transport = StreamableHttpClientTransport::from_config(transport_config);
        let client = ()
            .serve(transport)
            .await
            .map_err(|err| {
                let error_chain = format_error_chain(&err);
                error!(
                    remote_url = %remote_url,
                    error = %err,
                    error_debug = ?err,
                    error_chain = %error_chain,
                    "failed to initialize remote MCP client",
                );
                format!(
                    "failed to connect to remote MCP server `{remote_url}` while sending initialize request: {error_chain}"
                )
            })?;

        let result = client
            .call_tool(CallToolRequestParams::new(name.to_owned()).with_arguments(arguments))
            .await
            .map_err(|err| {
                let error_chain = format_error_chain(&err);
                error!(
                    remote_url = %remote_url,
                    tool_name = %name,
                    error = %err,
                    error_debug = ?err,
                    error_chain = %error_chain,
                    "remote MCP tool call failed",
                );
                format!("remote tool `{name}` failed via `{remote_url}`: {error_chain}")
            })?;

        if let Err(err) = client.cancel().await {
            let error_chain = format_error_chain(&err);
            error!(
                remote_url = %remote_url,
                error = %err,
                error_debug = ?err,
                error_chain = %error_chain,
                "failed to cancel remote MCP client session cleanly",
            );
        }

        Ok(result)
    }

    async fn call_remote_tool_typed<T>(
        &self,
        name: &str,
        arguments: serde_json::Map<String, serde_json::Value>,
        incoming_bearer_token: Option<&str>,
    ) -> Result<T, String>
    where
        T: DeserializeOwned,
    {
        let result = self
            .call_remote_tool(name, arguments, incoming_bearer_token)
            .await?;

        if result.is_error == Some(true) {
            return Err(remote_tool_error(name, &result));
        }

        result
            .into_typed()
            .map_err(|err| format!("failed to decode remote tool `{name}` result: {err}"))
    }

    async fn discover_project_ids_from_fallback_tools(
        &self,
        incoming_bearer_token: Option<&str>,
    ) -> Result<Vec<String>, String> {
        let probes = vec![
            (
                "list_undone_tasks_by_date",
                map_to_object(serde_json::json!({
                    // "search": {
                    //     "startDate": undone_start,
                    //     "endDate": undone_end,
                    // }
                }))?,
            ),
            (
                "list_completed_tasks_by_date",
                map_to_object(serde_json::json!({
                    // "search": {
                    //     "startDate": completed_start,
                    //     "endDate": completed_end,
                    // }
                }))?,
            ),
        ];

        let mut project_ids = BTreeSet::new();
        let mut probe_errors = Vec::new();

        for (tool_name, arguments) in probes {
            match self
                .call_remote_tool_typed::<Value>(tool_name, arguments, incoming_bearer_token)
                .await
            {
                Ok(value) => collect_project_ids(&value, &mut project_ids),
                Err(err) => {
                    warn!(
                        tool_name = %tool_name,
                        error = %err,
                        "fallback project id discovery probe failed",
                    );
                    probe_errors.push(format!("{tool_name}: {err}"));
                }
            }
        }

        if project_ids.is_empty() {
            match self
                .call_remote_tool_typed::<Value>(
                    "filter_tasks",
                    map_to_object(serde_json::json!({
                        "filter": {},
                    }))?,
                    incoming_bearer_token,
                )
                .await
            {
                Ok(value) => collect_project_ids(&value, &mut project_ids),
                Err(err) => {
                    warn!(
                        tool_name = "filter_tasks",
                        error = %err,
                        "broad fallback project id discovery probe failed",
                    );
                    probe_errors.push(format!("filter_tasks: {err}"));
                }
            }
        }

        if project_ids.is_empty() {
            if probe_errors.is_empty() {
                Err("fallback probes completed but no project_id was discovered".to_owned())
            } else {
                Err(format!(
                    "fallback probes did not yield any project_id: {}",
                    probe_errors.join(" ; ")
                ))
            }
        } else {
            Ok(project_ids.into_iter().collect())
        }
    }

    async fn project_profile_by_id_with_fallback(
        &self,
        project_id: &str,
        incoming_bearer_token: Option<&str>,
    ) -> Result<ProjectProfile, String> {
        self.call_remote_tool_typed(
            "get_project_by_id",
            map_to_object(serde_json::json!({
                "project_id": project_id,
            }))?,
            incoming_bearer_token,
        )
        .await
    }

    async fn list_projects_via_fallback(
        &self,
        incoming_bearer_token: Option<&str>,
        original_error: String,
    ) -> Result<CallToolResult, String> {
        warn!(
            error = %original_error,
            "list_projects failed, attempting project id fallback probes",
        );

        let project_ids = self
            .discover_project_ids_from_fallback_tools(incoming_bearer_token)
            .await
            .map_err(|fallback_error| format!("{original_error}; {fallback_error}"))?;

        let mut projects = Vec::with_capacity(project_ids.len());

        for project_id in project_ids {
            match self
                .project_profile_by_id_with_fallback(&project_id, incoming_bearer_token)
                .await
            {
                Ok(project) => projects.push(project),
                Err(err) => {
                    warn!(
                        project_id = %project_id,
                        error = %err,
                        "failed to fetch project details during list_projects fallback",
                    );
                    projects.push(ProjectProfile::from_id(project_id));
                }
            }
        }

        sort_project_profiles(&mut projects);

        Ok(CallToolResult::structured(serde_json::json!(
            ProjectListResult { result: projects }
        )))
    }

    fn resolve_remote_bearer_token(
        &self,
        incoming_bearer_token: Option<&str>,
    ) -> Result<Option<String>, String> {
        let configured_bearer = normalized_token(Some(self.config.remote.bearer_token.as_str()));

        match self.config.remote.bearer_mode {
            RemoteBearerMode::Fixed => configured_bearer
                .map(str::to_owned)
                .map(Some)
                .ok_or_else(|| {
                    "`remote.bearer_token` is required when `remote.bearer_mode = \"fixed\"`"
                        .to_owned()
                }),
            RemoteBearerMode::Passthrough => incoming_bearer_token
                .map(str::to_owned)
                .map(Some)
                .ok_or_else(|| {
                    format!(
                        "missing inbound Bearer token in `{}` for `remote.bearer_mode = \"passthrough\"`",
                        self.config.remote.incoming_bearer_header
                    )
                }),
            RemoteBearerMode::PassthroughOrFixed => incoming_bearer_token
                .or(configured_bearer)
                .map(str::to_owned)
                .map(Some)
                .ok_or_else(|| {
                    format!(
                        "missing inbound Bearer token in `{}` and `remote.bearer_token` fallback is empty",
                        self.config.remote.incoming_bearer_header
                    )
                }),
            RemoteBearerMode::None => Ok(None),
        }
    }

    fn resolve_timezone(&self, requested: Option<&str>) -> Result<Tz, String> {
        if let Some(timezone) = requested {
            return timezone
                .parse::<Tz>()
                .map_err(|_| format!("unsupported timezone: {timezone}"));
        }

        if let Some(timezone) = self.config.tools.default_timezone.as_deref() {
            return timezone
                .parse::<Tz>()
                .map_err(|_| format!("unsupported default timezone in config: {timezone}"));
        }

        Ok(chrono_tz::UTC)
    }

    fn incoming_bearer_token<'a>(&self, ctx: &'a RequestContext<RoleServer>) -> Option<&'a str> {
        let parts = ctx.extensions.get::<Parts>()?;
        extract_bearer_token(
            &parts.headers,
            self.config.remote.incoming_bearer_header.as_str(),
        )
    }
}

#[tool_router]
impl DidaProxy {
    #[tool(description = "List all projects of the current user.")]
    async fn list_projects(
        &self,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, String> {
        let incoming_bearer_token = self.incoming_bearer_token(&ctx);

        match self
            .call_remote_tool(
                "list_projects",
                serde_json::Map::new(),
                incoming_bearer_token,
            )
            .await
        {
            Ok(result) if result.is_error != Some(true) => Ok(result),
            Ok(result) => {
                self.list_projects_via_fallback(
                    incoming_bearer_token,
                    remote_tool_error("list_projects", &result),
                )
                .await
            }
            Err(err) => {
                self.list_projects_via_fallback(incoming_bearer_token, err)
                    .await
            }
        }
    }

    #[tool(description = "Get project details by project_id.")]
    async fn get_project_by_id(
        &self,
        Parameters(args): Parameters<ProjectIdArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, String> {
        self.call_remote_tool(
            "get_project_by_id",
            map_to_object(serde_json::json!({
                "project_id": args.project_id,
            }))?,
            self.incoming_bearer_token(&ctx),
        )
        .await
    }

    #[tool(description = "Get project details and undone tasks by project_id.")]
    async fn get_project_with_undone_tasks(
        &self,
        Parameters(args): Parameters<ProjectIdArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, String> {
        self.call_remote_tool(
            "get_project_with_undone_tasks",
            map_to_object(serde_json::json!({
                "project_id": args.project_id,
            }))?,
            self.incoming_bearer_token(&ctx),
        )
        .await
    }

    #[tool(description = "Create a task in Dida365 / TickTick.")]
    async fn create_task(
        &self,
        Parameters(args): Parameters<CreateTaskArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, String> {
        let remote_task = RemoteTaskPayload::from_create(args);
        self.call_remote_tool(
            "create_task",
            map_to_object(serde_json::json!({
                "task": remote_task,
            }))?,
            self.incoming_bearer_token(&ctx),
        )
        .await
    }

    #[tool(description = "Update an existing task by task_id.")]
    async fn update_task(
        &self,
        Parameters(args): Parameters<UpdateTaskArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, String> {
        let task_id = args.task_id.clone();
        let remote_task = RemoteTaskPayload::from_update(args);
        self.call_remote_tool(
            "update_task",
            map_to_object(serde_json::json!({
                "task_id": task_id,
                "task": remote_task,
            }))?,
            self.incoming_bearer_token(&ctx),
        )
        .await
    }

    #[tool(description = "Get full task details by task_id.")]
    async fn get_task_by_id(
        &self,
        Parameters(args): Parameters<TaskIdArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, String> {
        self.call_remote_tool(
            "get_task_by_id",
            map_to_object(serde_json::json!({
                "task_id": args.task_id,
            }))?,
            self.incoming_bearer_token(&ctx),
        )
        .await
    }

    #[tool(description = "Search tasks by keyword.")]
    async fn search_task(
        &self,
        Parameters(args): Parameters<SearchTaskArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, String> {
        self.call_remote_tool(
            "search_task",
            map_to_object(serde_json::json!({
                "query": args.query,
            }))?,
            self.incoming_bearer_token(&ctx),
        )
        .await
    }

    #[tool(description = "List undone tasks within a date range.")]
    async fn list_undone_tasks_by_date(
        &self,
        Parameters(args): Parameters<ListUndoneTasksByDateArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, String> {
        let search = RemoteUndoneTaskSearch {
            project_ids: args.project_ids,
            start_date: args.start_date,
            end_date: args.end_date,
        };

        self.call_remote_tool(
            "list_undone_tasks_by_date",
            map_to_object(serde_json::json!({
                "search": search,
            }))?,
            self.incoming_bearer_token(&ctx),
        )
        .await
    }

    #[tool(description = "Mark a task as completed by project_id and task_id.")]
    async fn complete_task(
        &self,
        Parameters(args): Parameters<CompleteTaskArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, String> {
        self.call_remote_tool(
            "complete_task",
            map_to_object(serde_json::json!({
                "project_id": args.project_id,
                "task_id": args.task_id,
            }))?,
            self.incoming_bearer_token(&ctx),
        )
        .await
    }

    #[tool(
        description = "Get the current time. You can optionally pass an IANA timezone like `Asia/Shanghai` or `America/Los_Angeles`."
    )]
    async fn get_current_time(
        &self,
        Parameters(args): Parameters<GetCurrentTimeArgs>,
    ) -> Result<McpJson<CurrentTimeResult>, String> {
        if !self.config.tools.enable_get_current_time {
            return Err("`get_current_time` is disabled in config.toml".to_owned());
        }

        let timezone = self.resolve_timezone(args.timezone.as_deref())?;
        let now_utc = Utc::now();
        let now_local = now_utc.with_timezone(&timezone);

        Ok(McpJson(CurrentTimeResult {
            timezone: timezone.name().to_owned(),
            iso_8601: now_local.to_rfc3339(),
            unix_timestamp: now_utc.timestamp(),
            date: now_local.date_naive().to_string(),
            time: now_local.time().format("%H:%M:%S").to_string(),
            utc_offset: now_local.offset().fix().to_string(),
        }))
    }
}

#[tool_handler(
    name = "dida-http-mcp-proxy",
    version = "0.1.0",
    instructions = "Task-focused Dida365 MCP proxy. It exposes a small set of task and project tools, forwards them to the configured remote MCP endpoint with bearer authentication, and provides a local get_current_time tool."
)]
impl ServerHandler for DidaProxy {}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::json;

    use super::{
        ProjectProfile, collect_project_ids, format_error_chain, remote_tool_error,
        sort_project_profiles,
    };
    use rmcp::model::CallToolResult;

    #[test]
    fn format_error_chain_lists_nested_sources() {
        let err = anyhow::anyhow!("tcp connect failed")
            .context("reqwest transport error")
            .context("initialize request failed");

        assert_eq!(
            format_error_chain(err.as_ref()),
            "initialize request failed | caused by: reqwest transport error | caused by: tcp connect failed"
        );
    }

    #[test]
    fn format_error_chain_skips_adjacent_duplicates() {
        let err = anyhow::Error::msg("same message").context("same message");

        assert_eq!(format_error_chain(err.as_ref()), "same message");
    }

    #[test]
    fn collect_project_ids_reads_nested_task_payloads() {
        let value = json!({
            "result": [
                {
                    "id": "task-1",
                    "projectId": "project-a",
                },
                {
                    "task": {
                        "project_id": "project-b",
                    }
                },
                {
                    "projectId": "project-a",
                }
            ]
        });

        let mut project_ids = BTreeSet::new();
        collect_project_ids(&value, &mut project_ids);

        assert_eq!(
            project_ids.into_iter().collect::<Vec<_>>(),
            vec!["project-a".to_owned(), "project-b".to_owned()]
        );
    }

    #[test]
    fn remote_tool_error_uses_structured_content_when_available() {
        let result = CallToolResult::structured_error(json!({
            "message": "list_projects is temporarily unavailable",
        }));

        assert_eq!(
            remote_tool_error("list_projects", &result),
            "remote tool `list_projects` returned an error result: {\"message\":\"list_projects is temporarily unavailable\"}"
        );
    }

    #[test]
    fn sort_project_profiles_prefers_sort_order_then_name() {
        let mut projects = vec![
            ProjectProfile {
                id: Some("project-c".to_owned()),
                name: Some("Gamma".to_owned()),
                sort_order: None,
                ..ProjectProfile::default()
            },
            ProjectProfile {
                id: Some("project-b".to_owned()),
                name: Some("Beta".to_owned()),
                sort_order: Some(20),
                ..ProjectProfile::default()
            },
            ProjectProfile {
                id: Some("project-a".to_owned()),
                name: Some("Alpha".to_owned()),
                sort_order: Some(10),
                ..ProjectProfile::default()
            },
        ];

        sort_project_profiles(&mut projects);

        assert_eq!(
            projects
                .into_iter()
                .map(|project| project.id.unwrap())
                .collect::<Vec<_>>(),
            vec![
                "project-a".to_owned(),
                "project-b".to_owned(),
                "project-c".to_owned()
            ]
        );
    }
}
