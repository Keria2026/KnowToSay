use crate::{
    db::AppState,
    error::{Error, Result},
    models::*,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn validate(folders: &[Folder], materials: &[Material]) -> Result<()> {
    let map: BTreeMap<_, _> = folders.iter().map(|f| (f.id.as_str(), f)).collect();
    if map.len() != folders.len() {
        return Err(Error::invalid("文件夹标识重复"));
    }
    let mut siblings = BTreeSet::new();
    for folder in folders {
        if folder.id.is_empty()
            || folder.id.len() > 100
            || folder.name.trim().is_empty()
            || folder.name.chars().count() > 80
            || folder.name.contains(['/', '\\'])
            || folder.name.chars().any(char::is_control)
        {
            return Err(Error::invalid(
                "文件夹名称需为 1–80 个字符，不能包含斜杠或控制字符",
            ));
        }
        if !siblings.insert((folder.parent_id.as_deref(), folder.name.to_lowercase())) {
            return Err(Error::conflict("同一位置已有同名文件夹"));
        }
        let mut seen = BTreeSet::from([folder.id.as_str()]);
        let mut parent = folder.parent_id.as_deref();
        while let Some(id) = parent {
            if !seen.insert(id) {
                return Err(Error::invalid("不能将文件夹移动到自身或子文件夹中"));
            }
            parent = map
                .get(id)
                .ok_or_else(|| Error::invalid("上级文件夹不存在"))?
                .parent_id
                .as_deref();
        }
    }
    if materials.iter().any(|m| {
        m.folder_id
            .as_deref()
            .is_some_and(|id| !map.contains_key(id))
    }) {
        return Err(Error::invalid("素材所属文件夹不存在"));
    }
    Ok(())
}

/// Only for old packages/backups that predate real folder identities.
pub fn upgrade(folders: &mut Vec<Folder>, materials: &mut [Material], categories: &[String]) {
    if !folders.is_empty() {
        return;
    }
    let names: BTreeSet<_> = categories
        .iter()
        .cloned()
        .chain(materials.iter().map(|m| m.category.trim().to_owned()))
        .filter(|s| !s.is_empty() && s != "未分类")
        .collect();
    for name in names {
        folders.push(Folder {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            parent_id: None,
        });
    }
    for material in materials {
        if material.folder_id.is_none() {
            material.folder_id = folders
                .iter()
                .find(|f| f.name == material.category.trim())
                .map(|f| f.id.clone());
        }
    }
}

impl AppState {
    pub async fn save_folder(&self, mut folder: Folder) -> Result<()> {
        let _guard = self.write.lock().await;
        self.admin().await?;
        folder.name = folder.name.trim().to_owned();
        if folder.id.is_empty() {
            folder.id = uuid::Uuid::new_v4().to_string();
        }
        let mut folders = self.list::<Folder>("draft_folders").await?;
        folders.retain(|f| f.id != folder.id);
        folders.push(folder.clone());
        validate(&folders, &[])?;
        self.put("draft_folders", &folder.id, &folder).await
    }
    pub async fn delete_folder(&self, id: &str) -> Result<()> {
        let _guard = self.write.lock().await;
        self.admin().await?;
        let mut folders = self.list::<Folder>("draft_folders").await?;
        let folder = folders
            .iter()
            .find(|f| f.id == id)
            .cloned()
            .ok_or_else(|| Error::missing("文件夹不存在"))?;
        folders.retain(|f| f.id != id);
        for child in &mut folders {
            if child.parent_id.as_deref() == Some(id) {
                child.parent_id = folder.parent_id.clone();
            }
        }
        validate(&folders, &[])?;
        let mut materials = self.list::<Material>("draft_materials").await?;
        for material in &mut materials {
            if material.folder_id.as_deref() == Some(id) {
                material.folder_id = folder.parent_id.clone();
                material.category.clear();
            }
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM draft_folders")
            .execute(&mut *tx)
            .await?;
        for f in folders {
            sqlx::query("INSERT INTO draft_folders(id,json) VALUES(?,?)")
                .bind(&f.id)
                .bind(serde_json::to_string(&f)?)
                .execute(&mut *tx)
                .await?;
        }
        for m in materials {
            sqlx::query("UPDATE draft_materials SET json=? WHERE id=?")
                .bind(serde_json::to_string(&m)?)
                .bind(&m.id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

pub fn decode_manifest(bytes: &[u8]) -> Result<Manifest> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    let legacy = value.get("folders").is_none();
    let mut manifest: Manifest = serde_json::from_value(value)?;
    if legacy {
        upgrade(
            &mut manifest.folders,
            &mut manifest.materials,
            &manifest.material_categories,
        );
    }
    manifest.upgrade();
    Ok(manifest)
}
