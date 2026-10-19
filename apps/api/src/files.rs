use crate::{
    db::{AppState, valid_hash},
    error::{Error, Result},
    models::*,
};
use axum::{
    body::Body,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use calamine::{Reader, open_workbook_auto};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_util::io::ReaderStream;

pub async fn hash_file(path: &Path) -> Result<String> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0; 1024 * 1024];
    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(hex::encode(hash.finalize()))
}

pub fn classify(name: &str) -> (&'static str, &'static str) {
    match name
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "png" => ("image", "image/png"),
        "jpg" | "jpeg" => ("image", "image/jpeg"),
        "webp" => ("image", "image/webp"),
        "gif" => ("image", "image/gif"),
        "mp4" => ("video", "video/mp4"),
        "webm" => ("video", "video/webm"),
        "mov" => ("video", "video/quicktime"),
        "xlsx" | "xls" | "xlsb" | "ods" => ("excel", "application/octet-stream"),
        "txt" | "md" | "csv" => ("text", "text/plain; charset=utf-8"),
        _ => ("file", "application/octet-stream"),
    }
}

pub async fn material_from_file(
    path: &Path,
    name: &str,
    hash: String,
    size: u64,
) -> Result<Material> {
    let (kind, mime) = classify(name);
    let mut material = Material {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.into(),
        kind: kind.into(),
        category: "未分类".into(),
        tags: vec![],
        notes: String::new(),
        text: String::new(),
        mime: mime.into(),
        hash: Some(hash),
        size,
        rows: vec![],
    };
    if kind == "text" && size <= 10 * 1024 * 1024 {
        material.text = tokio::fs::read_to_string(path)
            .await
            .map_err(|_| Error::invalid("文字文件需使用 UTF-8 编码"))?;
    }
    if kind == "excel" {
        let path = path.to_path_buf();
        material.rows = tokio::task::spawn_blocking(move || read_rows(&path))
            .await
            .map_err(Error::internal)??;
    }
    Ok(material)
}

fn read_rows(path: &Path) -> Result<Vec<SheetRow>> {
    let mut workbook =
        open_workbook_auto(path).map_err(|e| Error::invalid(format!("无法读取表格：{e}")))?;
    let mut rows = Vec::new();
    for (sheet, range) in workbook.worksheets() {
        let first = range.start().map_or(0, |p| p.0);
        for (i, row) in range.rows().enumerate() {
            if rows.len() >= 100_000 {
                return Err(Error::invalid("单个表格最多索引 100,000 行，请拆分后导入"));
            }
            let cells: Vec<_> = row.iter().map(|c| c.to_string()).collect();
            if cells.iter().any(|s| !s.is_empty()) {
                rows.push(SheetRow {
                    sheet: sheet.clone(),
                    row: first + i as u32 + 1,
                    cells,
                });
            }
        }
    }
    Ok(rows)
}

pub async fn stream_blob(
    state: &AppState,
    hash: &str,
    size: u64,
    mime: &str,
    headers: &HeaderMap,
    attachment: bool,
) -> Result<Response> {
    if !valid_hash(hash) {
        return Err(Error::missing("素材已移除"));
    }
    let path = state.root.join("blobs").join(hash);
    let mut file = tokio::fs::File::open(&path)
        .await
        .map_err(|_| Error::missing("本地素材文件缺失，请重新同步"))?;
    if file.metadata().await?.len() != size {
        return Err(Error::conflict("本地素材文件长度异常，请重新校验同步"));
    }
    let range = headers.get(header::RANGE).and_then(|v| v.to_str().ok());
    let (start, end, partial) = match range {
        None => (0, size.saturating_sub(1), false),
        Some(value) => {
            let (start, end) = value
                .strip_prefix("bytes=")
                .and_then(|v| v.split_once('-'))
                .ok_or_else(|| Error::invalid("不支持的分段请求"))?;
            let start: u64 = start.parse().map_err(|_| Error::invalid("分段起点无效"))?;
            let end = if end.is_empty() {
                size.saturating_sub(1)
            } else {
                end.parse().map_err(|_| Error::invalid("分段终点无效"))?
            };
            if start >= size || end < start || end >= size {
                return Ok((
                    StatusCode::RANGE_NOT_SATISFIABLE,
                    [(header::CONTENT_RANGE, format!("bytes */{size}"))],
                )
                    .into_response());
            }
            (start, end, true)
        }
    };
    file.seek(std::io::SeekFrom::Start(start)).await?;
    let length = if size == 0 { 0 } else { end - start + 1 };
    let body = Body::from_stream(ReaderStream::new(file.take(length)));
    let mut builder = Response::builder()
        .status(if partial {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        })
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, length.to_string())
        .header("x-content-type-options", "nosniff")
        .header(header::CACHE_CONTROL, "no-store");
    if partial {
        builder = builder.header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{size}"))
    }
    if attachment {
        builder = builder.header(header::CONTENT_DISPOSITION, "attachment")
    }
    builder.body(body).map_err(Error::internal)
}

pub async fn import_paths(state: &AppState, paths: Vec<String>) -> Result<Vec<Material>> {
    let _guard = state.write.lock().await;
    state.admin().await?;
    let mut output = Vec::new();
    for source in paths {
        let source = std::path::PathBuf::from(source);
        let name = source
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| Error::invalid("文件名无效"))?
            .to_string();
        if !source.is_file() {
            return Err(Error::invalid("请选择普通文件"));
        }
        let size = tokio::fs::metadata(&source).await?.len();
        if fs2::available_space(&*state.root)? < size + 64 * 1024 * 1024 {
            return Err(Error::conflict("磁盘空间不足，无法导入素材"));
        }
        let hash = hash_file(&source).await?;
        let target = state.root.join("blobs").join(&hash);
        let staging = state
            .root
            .join("staging")
            .join(uuid::Uuid::new_v4().to_string());
        tokio::fs::copy(&source, &staging).await?;
        if hash_file(&staging).await? != hash {
            tokio::fs::remove_file(staging).await?;
            return Err(Error::conflict("导入期间原文件发生变化，请重试"));
        }
        // Read Excel with its original extension before moving to a content-addressed path.
        let with_extension = staging.with_extension(source.extension().unwrap_or_default());
        tokio::fs::rename(&staging, &with_extension).await?;
        let result = material_from_file(&with_extension, &name, hash.clone(), size).await;
        let material = match result {
            Ok(m) => m,
            Err(e) => {
                let _ = tokio::fs::remove_file(&with_extension).await;
                return Err(e);
            }
        };
        if target.exists()
            && tokio::fs::metadata(&target).await?.len() == size
            && hash_file(&target).await? == hash
        {
            tokio::fs::remove_file(&with_extension).await?;
        } else {
            if target.exists() {
                tokio::fs::remove_file(&target).await?;
            }
            tokio::fs::rename(with_extension, target).await?;
        }
        state
            .put("draft_materials", &material.id, &material)
            .await?;
        output.push(material);
    }
    Ok(output)
}

pub async fn upload(
    state: &AppState,
    mut multipart: axum::extract::Multipart,
) -> Result<Vec<Material>> {
    let _guard = state.write.lock().await;
    state.admin().await?;
    let mut output = Vec::new();
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| Error::invalid(e.to_string()))?
    {
        let name = field
            .file_name()
            .ok_or_else(|| Error::invalid("上传文件需要文件名"))?
            .to_owned();
        let extension = Path::new(&name)
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("bin");
        if extension.len() > 12 || !extension.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(Error::invalid("文件扩展名无效"));
        }
        let path =
            state
                .root
                .join("staging")
                .join(format!("{}.{}", uuid::Uuid::new_v4(), extension));
        let mut file = tokio::fs::File::create(&path).await?;
        let mut hash = Sha256::new();
        let mut size = 0u64;
        while let Some(bytes) = field
            .chunk()
            .await
            .map_err(|e| Error::invalid(e.to_string()))?
        {
            if fs2::available_space(&*state.root)? < bytes.len() as u64 + 64 * 1024 * 1024 {
                drop(file);
                let _ = tokio::fs::remove_file(&path).await;
                return Err(Error::conflict("磁盘空间不足"));
            }
            size += bytes.len() as u64;
            hash.update(&bytes);
            file.write_all(&bytes).await?;
        }
        file.sync_all().await?;
        drop(file);
        let hash = hex::encode(hash.finalize());
        let material = match material_from_file(&path, &name, hash.clone(), size).await {
            Ok(m) => m,
            Err(e) => {
                let _ = tokio::fs::remove_file(&path).await;
                return Err(e);
            }
        };
        let target = state.root.join("blobs").join(hash);
        if target.exists() {
            tokio::fs::remove_file(&path).await?
        } else {
            tokio::fs::rename(&path, target).await?
        }
        state
            .put("draft_materials", &material.id, &material)
            .await?;
        output.push(material);
    }
    Ok(output)
}

pub fn scripts_from_rows(
    rows: &[SheetRow],
    mapping: &BTreeMap<String, String>,
) -> Result<Vec<ScriptNode>> {
    let header = rows.first().ok_or_else(|| Error::invalid("表格为空"))?;
    let column = |key: &str| {
        header
            .cells
            .iter()
            .position(|c| c.trim() == mapping.get(key).map_or(key, |v| v.as_str()))
    };
    let id_col = column("id")
        .ok_or_else(|| Error::invalid("缺少 id 列，请为每条话术提供稳定且唯一的 ID"))?;
    let title_col =
        column("title").ok_or_else(|| Error::invalid("缺少 title 列，请选择字段映射"))?;
    let mut nodes = Vec::new();
    let mut ids = std::collections::BTreeSet::new();
    for row in rows.iter().skip(1).filter(|r| r.sheet == header.sheet) {
        let get = |key: &str| {
            column(key)
                .and_then(|i| row.cells.get(i))
                .cloned()
                .unwrap_or_default()
        };
        let id = row.cells.get(id_col).cloned().unwrap_or_default();
        let title = row.cells.get(title_col).cloned().unwrap_or_default();
        if id.trim().is_empty() || !ids.insert(id.clone()) {
            return Err(Error::invalid(format!("第 {} 行 id 为空或重复", row.row)));
        }
        let mut replies = BTreeMap::new();
        for (i, name) in header.cells.iter().enumerate() {
            if let Some(lang) = name.strip_prefix("reply:") {
                replies.insert(
                    lang.to_owned(),
                    row.cells.get(i).cloned().unwrap_or_default(),
                );
            }
        }
        for (target, source) in mapping.iter().filter(|(k, _)| k.starts_with("reply:")) {
            if let Some(i) = header.cells.iter().position(|c| c == source) {
                replies.insert(
                    target[6..].into(),
                    row.cells.get(i).cloned().unwrap_or_default(),
                );
            }
        }
        let parse = |key: &str| {
            let value = get(key);
            if value.trim().is_empty() {
                "[]".into()
            } else {
                value
            }
        };
        let n = ScriptNode {
            flow_id: None,
            id,
            title,
            question: get("question"),
            category: get("category"),
            kind: match get("kind").trim() {
                "" | "flow" | "流程话术" => ScriptKind::Flow,
                "quick" | "快捷话术" => ScriptKind::Quick,
                _ => {
                    return Err(Error::invalid(format!(
                        "第 {} 行话术类型需为 flow 或 quick",
                        row.row
                    )));
                }
            },
            is_entry: matches!(get("isEntry").as_str(), "true" | "1" | "是"),
            replies,
            variables: serde_json::from_str(&parse("variables")).map_err(|_| {
                Error::invalid(format!("第 {} 行 variables 不是有效 JSON", row.row))
            })?,
            branches: serde_json::from_str(&parse("branches"))
                .map_err(|_| Error::invalid(format!("第 {} 行 branches 不是有效 JSON", row.row)))?,
            material_ids: serde_json::from_str(&parse("materialIds")).map_err(|_| {
                Error::invalid(format!("第 {} 行 materialIds 不是有效 JSON", row.row))
            })?,
            position: None,
        };
        crate::db::validate_node(&n)?;
        nodes.push(n);
    }
    if nodes.is_empty() {
        return Err(Error::invalid("未找到可导入的话术行"));
    }
    Ok(nodes)
}
