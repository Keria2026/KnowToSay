use crate::{
    db::{AppState, validate_manifest_content},
    error::{Error, Result},
    files::hash_file,
    models::*,
};
use futures_util::StreamExt;
use std::{
    collections::BTreeMap,
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use tokio::{io::AsyncWriteExt, net::UdpSocket};

pub const LAN_PORT: u16 = 51876;
pub const DISCOVERY_PORT: u16 = 51877;

pub fn normalize_address(input: &str) -> Result<String> {
    let address = if input.contains("://") {
        input.to_string()
    } else {
        format!("http://{input}")
    };
    let url = reqwest::Url::parse(&address)
        .map_err(|_| Error::invalid("请输入管理员的局域网地址，如 http://192.168.1.10:51876"))?;
    let ip: IpAddr = url
        .host_str()
        .ok_or_else(|| Error::invalid("管理员地址无效"))?
        .parse()
        .map_err(|_| Error::invalid("请使用局域网 IPv4 地址，不使用域名"))?;
    let local = match ip {
        IpAddr::V4(ip) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
        IpAddr::V6(_) => false,
    };
    if url.scheme() != "http"
        || !local
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(Error::invalid("只支持无用户名和路径的局域网 HTTP 地址"));
    }
    Ok(format!("http://{}:{}", ip, url.port().unwrap_or(LAN_PORT)))
}

pub async fn fetch_manifest(state: &AppState, address: &str) -> Result<Manifest> {
    let response = state
        .client
        .get(format!("{address}/api/v1/manifest"))
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(Error::conflict(
            "管理员尚未发布资料，或此地址不是工作组同步服务",
        ));
    }
    if response
        .content_length()
        .is_some_and(|n| n > 64 * 1024 * 1024)
    {
        return Err(Error::invalid("资料清单超过大小限制"));
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        bytes.extend_from_slice(&chunk?);
        if bytes.len() > 64 * 1024 * 1024 {
            return Err(Error::invalid("资料清单超过大小限制"));
        }
    }
    let manifest = crate::folders::decode_manifest(&bytes)?;
    if manifest.format_version != 2
        || manifest.version < 1
        || uuid::Uuid::parse_str(&manifest.group_id).is_err()
    {
        return Err(Error::invalid("管理员资料清单无效"));
    }
    validate_manifest_content(&manifest.nodes, &manifest.materials)?;
    crate::db::validate_flow_membership(&manifest.nodes, &manifest.flows)?;
    Ok(manifest)
}

pub async fn synchronize(state: &AppState, address: &str) -> Result<()> {
    let _guard = state
        .sync
        .try_lock()
        .map_err(|_| Error::conflict("已有同步任务正在运行"))?;
    let address = normalize_address(address)?;
    *state.progress.write().await = SyncStatus {
        phase: "checking".into(),
        message: "正在检查管理员发布版本".into(),
        ..Default::default()
    };
    let result = sync_inner(state, &address).await;
    if let Err(e) = &result {
        let mut p = state.progress.write().await;
        p.phase = "failed".into();
        p.message = e.2.clone()
    }
    result
}

async fn sync_inner(state: &AppState, address: &str) -> Result<()> {
    let manifest = fetch_manifest(state, address).await?;
    if let Some(w) = state.workspace().await? {
        if w.role != Role::Sales || w.group_id != manifest.group_id {
            return Err(Error::conflict("此管理员不属于当前工作组"));
        }
        if manifest.version <= w.version {
            *state.progress.write().await = SyncStatus {
                phase: "current".into(),
                message: "当前资料已是最新发布版本".into(),
                version: w.version,
                ..Default::default()
            };
            return Ok(());
        }
    }
    let mut needed = BTreeMap::new();
    for m in &manifest.materials {
        if let Some(hash) = &m.hash
            && let Some(old) = needed.insert(hash.clone(), m.size)
            && old != m.size
        {
            return Err(Error::invalid("同一素材校验值对应了不同文件长度"));
        }
    }
    let mut total = 0u64;
    let mut additional = 0u64;
    for (hash, size) in &needed {
        if !tokio::fs::metadata(state.root.join("blobs").join(hash))
            .await
            .is_ok_and(|m| m.len() == *size)
        {
            total = total
                .checked_add(*size)
                .ok_or_else(|| Error::invalid("资料大小超过限制"))?;
            let part_size =
                tokio::fs::metadata(state.root.join("staging").join(format!("{hash}.part")))
                    .await
                    .map_or(0, |m| m.len().min(*size));
            additional = additional
                .checked_add(size - part_size)
                .ok_or_else(|| Error::invalid("资料大小超过限制"))?;
        }
    }
    if fs2::available_space(&*state.root)? < additional.saturating_add(128 * 1024 * 1024) {
        return Err(Error::conflict(
            "磁盘空间不足以安全完成本次更新，已保留旧版本",
        ));
    }
    *state.progress.write().await = SyncStatus {
        phase: "downloading".into(),
        message: "正在增量下载素材".into(),
        total_bytes: total,
        version: manifest.version,
        ..Default::default()
    };
    for (hash, size) in needed {
        let target = state.root.join("blobs").join(&hash);
        if tokio::fs::metadata(&target)
            .await
            .is_ok_and(|m| m.len() == size)
        {
            continue;
        }
        let part = state.root.join("staging").join(format!("{hash}.part"));
        let mut offset = tokio::fs::metadata(&part)
            .await
            .map(|m| m.len())
            .unwrap_or(0);
        if offset > size {
            tokio::fs::remove_file(&part).await?;
            offset = 0
        }
        state.progress.write().await.completed_bytes += offset;
        if offset < size || size == 0 {
            let mut req = state.client.get(format!("{address}/api/v1/blobs/{hash}"));
            if offset > 0 {
                req = req.header("range", format!("bytes={offset}-"))
            }
            let response = req.send().await?;
            if !response.status().is_success() {
                return Err(Error::conflict("管理员素材当前不可用，请稍后重试"));
            }
            if offset > 0 && response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
                let expected = format!("bytes {offset}-{}", size.saturating_sub(1));
                let actual = response
                    .headers()
                    .get("content-range")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");
                if actual != format!("{expected}/{size}") {
                    return Err(Error::conflict("素材续传范围不一致，请重试"));
                }
            } else {
                let mut p = state.progress.write().await;
                p.completed_bytes = p.completed_bytes.saturating_sub(offset);
                offset = 0
            }
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(offset == 0)
                .append(offset > 0)
                .open(&part)
                .await?;
            let mut stream = response.bytes_stream();
            let mut written = offset;
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                written += chunk.len() as u64;
                if written > size {
                    drop(file);
                    let _ = tokio::fs::remove_file(&part).await;
                    return Err(Error::conflict("素材传输长度异常，已保留旧版本"));
                }
                file.write_all(&chunk).await?;
                state.progress.write().await.completed_bytes += chunk.len() as u64;
            }
            file.sync_all().await?;
            drop(file);
        }
        state.progress.write().await.message = "正在校验下载文件".into();
        if tokio::fs::metadata(&part).await?.len() != size || hash_file(&part).await? != hash {
            let _ = tokio::fs::remove_file(&part).await;
            return Err(Error::conflict(
                "素材完整性校验失败，已丢弃损坏下载并保留旧版本，请重试",
            ));
        }
        if target.exists() {
            tokio::fs::remove_file(&target).await?
        }
        tokio::fs::rename(part, target).await?;
    }
    state.progress.write().await.phase = "applying".into();
    state.apply(&manifest, address).await?;
    *state.progress.write().await = SyncStatus {
        phase: "current".into(),
        message: "已同步最新发布版本，个人修改保持不变".into(),
        version: manifest.version,
        completed_bytes: total,
        total_bytes: total,
    };
    Ok(())
}

pub async fn discover() -> Result<Vec<Discovery>> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.set_broadcast(true)?;
    socket
        .send_to(
            b"KNOWTOSAY_DISCOVER_V1",
            format!("255.255.255.255:{DISCOVERY_PORT}"),
        )
        .await?;
    let mut buffer = [0u8; 4096];
    let deadline = tokio::time::Instant::now() + Duration::from_millis(1200);
    let mut found = BTreeMap::new();
    while let Ok(Ok((n, from))) =
        tokio::time::timeout_at(deadline, socket.recv_from(&mut buffer)).await
    {
        if let Ok(mut d) = serde_json::from_slice::<Discovery>(&buffer[..n])
            && from.ip().is_ipv4()
            && uuid::Uuid::parse_str(&d.group_id).is_ok()
        {
            d.address = format!("http://{}:{LAN_PORT}", from.ip());
            found.insert(d.group_id.clone(), d);
        }
    }
    Ok(found.into_values().collect())
}

pub async fn discovery_server(state: AppState) -> Result<()> {
    let socket = UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], DISCOVERY_PORT))).await?;
    let mut buffer = [0u8; 256];
    loop {
        let (n, from) = socket.recv_from(&mut buffer).await?;
        if &buffer[..n] != b"KNOWTOSAY_DISCOVER_V1" {
            continue;
        }
        if let Some(w) = state.workspace().await?
            && w.role == Role::Admin
        {
            let d = Discovery {
                group_id: w.group_id,
                name: w.name,
                address: String::new(),
            };
            socket.send_to(&serde_json::to_vec(&d)?, from).await?;
        }
    }
}

pub fn background(state: AppState) {
    let discovery = state.clone();
    tokio::spawn(async move {
        if let Err(e) = discovery_server(discovery).await {
            eprintln!("局域网发现不可用: {}", e.2)
        }
    });
    let lan = state.clone();
    tokio::spawn(async move {
        match tokio::net::TcpListener::bind(("0.0.0.0", LAN_PORT)).await {
            Ok(listener) => {
                if let Err(e) = axum::serve(listener, crate::http::lan_router(lan)).await {
                    eprintln!("局域网同步服务: {e}")
                }
            }
            Err(e) => eprintln!("局域网端口不可用: {e}"),
        }
    });
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(20));
        loop {
            interval.tick().await;
            if let Ok(Some(w)) = state.workspace().await
                && w.role == Role::Sales
                && let Some(address) = w.admin_address
            {
                let _ = synchronize(&state, &address).await;
            }
        }
    });
}
