use crate::{
    db::AppState,
    error::{Error, Result},
    models::*,
};
use futures_util::TryStreamExt;

pub async fn search(state: &AppState, query: &str) -> Result<Vec<MaterialSearchResult>> {
    search_filtered(state, query, None).await
}
pub async fn search_filtered(
    state: &AppState,
    query: &str,
    category: Option<&str>,
) -> Result<Vec<MaterialSearchResult>> {
    search_folder(state, query, category, None).await
}
pub async fn search_folder(
    state: &AppState,
    query: &str,
    category: Option<&str>,
    folder: Option<&str>,
) -> Result<Vec<MaterialSearchResult>> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Ok(vec![]);
    }
    if query.len() > 500 {
        return Err(Error::invalid("搜索词过长"));
    }
    let admin = state.required_workspace().await?.role == Role::Admin;
    let table = if admin {
        "draft_materials"
    } else {
        "official_materials"
    };
    let folders = state
        .list::<Folder>(if admin {
            "draft_folders"
        } else {
            "official_folders"
        })
        .await?;
    let mut folder_ids = std::collections::BTreeSet::new();
    if let Some(id) = folder {
        folder_ids.insert(id.to_owned());
        loop {
            let before = folder_ids.len();
            for f in &folders {
                if f.parent_id
                    .as_ref()
                    .is_some_and(|parent| folder_ids.contains(parent))
                {
                    folder_ids.insert(f.id.clone());
                }
            }
            if before == folder_ids.len() {
                break;
            }
        }
    }
    let source = if admin { "draft" } else { "official" };
    let (sql, argument) = if query.chars().count() >= 3 {
        (
            format!(
                "SELECT m.json FROM {table} m JOIN material_search s ON s.material_id=m.id WHERE s.source='{source}' AND material_search MATCH ? ORDER BY rank"
            ),
            format!("\"{}\"", query.replace('"', "\"\"")),
        )
    } else {
        (
            format!("SELECT json FROM {table} WHERE instr(lower(json),lower(?))>0 ORDER BY id"),
            query.clone(),
        )
    };
    let mut stream = sqlx::query_scalar::<_, String>(&sql)
        .bind(argument)
        .fetch(&state.pool);
    let mut results = Vec::new();
    while let Some(json) = stream.try_next().await? {
        let mut material: Material = serde_json::from_str(&json)?;
        if folder.is_some()
            && !material
                .folder_id
                .as_ref()
                .is_some_and(|id| folder_ids.contains(id))
        {
            continue;
        }
        if category.is_some_and(|c| {
            c != if material.category.trim().is_empty() {
                "未分类"
            } else {
                material.category.trim()
            }
        }) {
            continue;
        }
        let direct = [
            &material.name,
            &material.category,
            &material.notes,
            &material.text,
        ]
        .into_iter()
        .chain(material.tags.iter())
        .any(|s| s.to_lowercase().contains(&query));
        let rows: Vec<_> = material
            .rows
            .iter()
            .filter(|r| r.cells.iter().any(|c| c.to_lowercase().contains(&query)))
            .take(15)
            .cloned()
            .collect();
        if !direct && rows.is_empty() {
            continue;
        }
        material.rows.truncate(150);
        results.push(MaterialSearchResult { material, rows });
        if results.len() >= 100 {
            break;
        }
    }
    Ok(results)
}

pub async fn rows(
    state: &AppState,
    id: &str,
    query: &str,
    offset: usize,
    sheet: Option<&str>,
) -> Result<SheetPage> {
    let table = if state.required_workspace().await?.role == Role::Admin {
        "draft_materials"
    } else {
        "official_materials"
    };
    let json: Option<String> = sqlx::query_scalar(&format!("SELECT json FROM {table} WHERE id=?"))
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;
    let material: Material =
        serde_json::from_str(&json.ok_or_else(|| Error::missing("素材已移除"))?)?;
    let q = query.trim().to_lowercase();
    let mut sheets = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut column_count = 0;
    for row in &material.rows {
        if seen.insert(&row.sheet) {
            sheets.push(row.sheet.clone());
        }
        if sheet.is_none_or(|s| s == row.sheet) {
            column_count = column_count.max(row.cells.len() as u32);
        }
    }
    let matches: Vec<_> = material
        .rows
        .into_iter()
        .filter(|r| {
            sheet.is_none_or(|s| s == r.sheet)
                && (q.is_empty() || r.cells.iter().any(|c| c.to_lowercase().contains(&q)))
        })
        .collect();
    let total = matches.len() as u32;
    Ok(SheetPage {
        sheets,
        column_count,
        rows: matches.into_iter().skip(offset).take(150).collect(),
        total,
    })
}
