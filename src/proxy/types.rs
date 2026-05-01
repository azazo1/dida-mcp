use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectProfile {
    pub(crate) id: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) color: Option<String>,
    pub(crate) sort_order: Option<i64>,
    pub(crate) closed: Option<bool>,
    pub(crate) group_id: Option<String>,
    pub(crate) view_mode: Option<String>,
    pub(crate) permission: Option<String>,
    pub(crate) kind: Option<String>,
}

impl ProjectProfile {
    pub(crate) fn from_id(project_id: impl Into<String>) -> Self {
        Self {
            id: Some(project_id.into()),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ProjectListResult {
    pub(crate) result: Vec<ProjectProfile>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub(crate) struct ProjectIdArgs {
    pub(crate) project_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub(crate) struct TaskIdArgs {
    pub(crate) task_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub(crate) struct SearchTaskArgs {
    pub(crate) query: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub(crate) struct CompleteTaskArgs {
    pub(crate) project_id: String,
    pub(crate) task_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub(crate) struct GetCurrentTimeArgs {
    #[schemars(description = "Optional IANA timezone name, for example `Asia/Shanghai`.")]
    pub(crate) timezone: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub(crate) struct ListUndoneTasksByDateArgs {
    #[schemars(description = "Optional project IDs to filter by.")]
    pub(crate) project_ids: Option<Vec<String>>,
    #[schemars(description = "Optional RFC 3339 start time.")]
    pub(crate) start_date: Option<String>,
    #[schemars(description = "Optional RFC 3339 end time.")]
    pub(crate) end_date: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub(crate) struct ChecklistItemInput {
    pub(crate) title: String,
    #[schemars(description = "0 means not completed, 1 means completed.")]
    pub(crate) status: Option<i64>,
    #[schemars(description = "Optional RFC 3339 start time for the checklist item.")]
    pub(crate) start_date: Option<String>,
    pub(crate) is_all_day: Option<bool>,
    pub(crate) time_zone: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub(crate) struct CreateTaskArgs {
    pub(crate) project_id: Option<String>,
    pub(crate) title: String,
    pub(crate) content: Option<String>,
    pub(crate) desc: Option<String>,
    pub(crate) start_date: Option<String>,
    pub(crate) due_date: Option<String>,
    pub(crate) time_zone: Option<String>,
    pub(crate) is_all_day: Option<bool>,
    #[schemars(description = "0 = none, 1 = low, 3 = medium, 5 = high.")]
    pub(crate) priority: Option<i64>,
    #[schemars(description = "Reminder triggers in TickTick TRIGGER format.")]
    pub(crate) reminders: Option<Vec<String>>,
    #[schemars(description = "RRULE or ERULE recurrence string.")]
    pub(crate) repeat_flag: Option<String>,
    pub(crate) checklist_items: Option<Vec<ChecklistItemInput>>,
    pub(crate) tags: Option<Vec<String>>,
    pub(crate) column_id: Option<String>,
    pub(crate) parent_id: Option<String>,
    #[schemars(description = "Usually `TEXT`, `NOTE`, or `CHECKLIST`.")]
    pub(crate) kind: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub(crate) struct UpdateTaskArgs {
    pub(crate) task_id: String,
    pub(crate) project_id: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) content: Option<String>,
    pub(crate) desc: Option<String>,
    pub(crate) start_date: Option<String>,
    pub(crate) due_date: Option<String>,
    pub(crate) time_zone: Option<String>,
    pub(crate) is_all_day: Option<bool>,
    pub(crate) priority: Option<i64>,
    pub(crate) reminders: Option<Vec<String>>,
    pub(crate) repeat_flag: Option<String>,
    pub(crate) checklist_items: Option<Vec<ChecklistItemInput>>,
    pub(crate) tags: Option<Vec<String>>,
    pub(crate) column_id: Option<String>,
    pub(crate) parent_id: Option<String>,
    pub(crate) kind: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoteTaskPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) desc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) due_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) time_zone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) is_all_day: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) priority: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reminders: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) repeat_flag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "items")]
    pub(crate) checklist_items: Option<Vec<RemoteChecklistItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) column_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) kind: Option<String>,
}

impl RemoteTaskPayload {
    pub(crate) fn from_create(args: CreateTaskArgs) -> Self {
        Self {
            project_id: args.project_id,
            title: Some(args.title),
            content: args.content,
            desc: args.desc,
            start_date: args.start_date,
            due_date: args.due_date,
            time_zone: args.time_zone,
            is_all_day: args.is_all_day,
            priority: args.priority,
            reminders: args.reminders,
            repeat_flag: args.repeat_flag,
            checklist_items: args
                .checklist_items
                .map(|items| items.into_iter().map(RemoteChecklistItem::from).collect()),
            tags: args.tags,
            column_id: args.column_id,
            parent_id: args.parent_id,
            kind: args.kind,
        }
    }

    pub(crate) fn from_update(args: UpdateTaskArgs) -> Self {
        Self {
            project_id: args.project_id,
            title: args.title,
            content: args.content,
            desc: args.desc,
            start_date: args.start_date,
            due_date: args.due_date,
            time_zone: args.time_zone,
            is_all_day: args.is_all_day,
            priority: args.priority,
            reminders: args.reminders,
            repeat_flag: args.repeat_flag,
            checklist_items: args
                .checklist_items
                .map(|items| items.into_iter().map(RemoteChecklistItem::from).collect()),
            tags: args.tags,
            column_id: args.column_id,
            parent_id: args.parent_id,
            kind: args.kind,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoteChecklistItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) status: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) is_all_day: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) time_zone: Option<String>,
    pub(crate) title: String,
}

impl From<ChecklistItemInput> for RemoteChecklistItem {
    fn from(value: ChecklistItemInput) -> Self {
        Self {
            title: value.title,
            status: value.status,
            start_date: value.start_date,
            is_all_day: value.is_all_day,
            time_zone: value.time_zone,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoteUndoneTaskSearch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) project_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) end_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub(crate) struct CurrentTimeResult {
    pub(crate) timezone: String,
    pub(crate) iso_8601: String,
    pub(crate) unix_timestamp: i64,
    pub(crate) date: String,
    pub(crate) time: String,
    pub(crate) utc_offset: String,
}

pub(crate) fn map_to_object(
    value: serde_json::Value,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    match value {
        serde_json::Value::Object(map) => Ok(map),
        _ => Err("expected a JSON object".to_owned()),
    }
}
