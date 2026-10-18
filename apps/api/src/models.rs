use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub group_id: String,
    pub name: String,
    pub role: Role,
    pub version: i64,
    pub admin_address: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Sales,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    pub name: String,
    pub label: String,
    pub required: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Branch {
    pub id: String,
    pub label: String,
    pub target_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptNode {
    pub id: String,
    pub title: String,
    pub question: String,
    pub category: String,
    pub is_entry: bool,
    pub replies: BTreeMap<String, String>,
    pub variables: Vec<Variable>,
    pub branches: Vec<Branch>,
    pub material_ids: Vec<String>,
    pub position: Option<Position>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SheetRow {
    pub sheet: String,
    pub row: u32,
    pub cells: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Material {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub category: String,
    pub tags: Vec<String>,
    pub notes: String,
    pub text: String,
    pub mime: String,
    pub hash: Option<String>,
    pub size: u64,
    pub rows: Vec<SheetRow>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NodeRecord {
    pub node: ScriptNode,
    pub source: String,
    pub official: Option<ScriptNode>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MaterialSearchResult {
    pub material: Material,
    pub rows: Vec<SheetRow>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SheetPage {
    pub rows: Vec<SheetRow>,
    pub total: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub workspace: Option<Workspace>,
    pub nodes: Vec<NodeRecord>,
    pub materials: Vec<Material>,
    #[serde(default)]
    pub material_categories: Vec<String>,
    pub draft_dirty: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub format_version: u32,
    pub group_id: String,
    pub group_name: String,
    pub version: i64,
    pub nodes: Vec<ScriptNode>,
    pub materials: Vec<Material>,
    #[serde(default)]
    pub material_categories: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateMaterialCategory {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateGroup {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JoinGroup {
    pub address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Discovery {
    pub group_id: String,
    pub name: String,
    pub address: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub phase: String,
    pub message: String,
    pub completed_bytes: u64,
    pub total_bytes: u64,
    pub version: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupRequest {
    pub directory: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RestoreRequest {
    pub directory: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FileResult {
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub request_id: String,
    pub details: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DesktopPreferences {
    pub shortcut: String,
    pub edges: Vec<String>,
    pub dwell_ms: u64,
    pub copy_hides: bool,
}
impl Default for DesktopPreferences {
    fn default() -> Self {
        Self {
            shortcut: "Ctrl+Alt+Space".into(),
            edges: vec!["right".into()],
            dwell_ms: 400,
            copy_hides: true,
        }
    }
}
impl DesktopPreferences {
    pub fn validate(&self) -> std::result::Result<(), String> {
        if self.shortcut.len() > 100
            || !(200..=2000).contains(&self.dwell_ms)
            || self
                .edges
                .iter()
                .any(|s| !["left", "right", "top", "bottom"].contains(&s.as_str()))
        {
            return Err("快捷键、触发边缘或停留时间无效（200–2000ms）".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DesktopStatus {
    pub preferences: DesktopPreferences,
    pub shortcut_registered: bool,
    pub warning: Option<String>,
}
