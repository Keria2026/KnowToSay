<script lang="ts">
  import { onMount } from 'svelte';
  import { RefreshCw, Upload, Radio } from 'lucide-svelte';
  import { app } from './store.svelte';
  import { api } from './api';
  import { bytes } from '$lib/utils/scripts';
  import type { Manifest } from '$lib/types';
  let published = $state<Manifest | null>(null);
  let address = $state(app.workspace?.adminAddress || '');
  onMount(() => {
    if (app.admin && app.workspace?.version)
      api<Manifest>('/manifest')
        .then((value) => (published = value))
        .catch(() => {});
  });
  async function publish() {
    await app.task(
      '正在发布',
      async () => {
        published = await api<Manifest>('/publish', 'POST');
        await app.refresh();
      },
      '已发布，局域网内业务员将自动获取更新',
    );
  }
</script>

<div class="sync-hero">
  <span class="sync-orbit"><Radio size={31} strokeWidth={1.5} /></span>
  <div>
    <span class="eyebrow">{app.workspace?.name}</span>
    <h3>
      {app.admin
        ? app.data?.draftDirty
          ? '有草稿等待发布'
          : app.workspace?.version
            ? '已发布资料保持一致'
            : '准备你的第一次发布'
        : app.progress?.phase === 'failed'
          ? '暂时无法同步，本地资料可用'
          : app.progress?.phase === 'downloading'
            ? '正在接收新资料'
            : '你的资料，随时可用'}
    </h3>
    <p class="muted">
      当前已发布版本 v{app.workspace?.version || 0}{app.admin
        ? ' · 草稿不会自动同步'
        : ' · 个人话术独立保存'}
    </p>
  </div>
  {#if app.admin}<button
      class="button primary"
      disabled={!!app.busy || !app.data?.draftDirty}
      onclick={publish}><Upload size={16} />发布更新</button
    >{:else}<button
      class="button primary"
      disabled={!!app.busy ||
        ['checking', 'downloading', 'applying'].includes(
          app.progress?.phase || '',
        )}
      onclick={() => app.task('检查更新', () => api('/sync', 'POST'))}
      ><RefreshCw size={16} />检查更新</button
    >{/if}
</div>
{#if !app.admin}<div class="settings-card">
    <h3>同步状态</h3>
    <p>{app.progress?.message || '等待与管理员建立连接'}</p>
    {#if app.progress?.totalBytes}<progress
        value={app.progress.completedBytes}
        max={app.progress.totalBytes}
      ></progress>
      <p class="field-help">
        {bytes(app.progress.completedBytes)} / {bytes(app.progress.totalBytes)}
      </p>{/if}
    <p class="field-help">
      下载并校验完成后统一切换版本。断线保留进度，更新失败继续使用原资料。
    </p>
  </div>{/if}
{#if app.admin}<div class="settings-card">
    <div class="section-title">
      <h3>本次发布内容</h3>
      <span class="pill"
        >{app.data?.draftDirty ? '未发布草稿' : '无待发布修改'}</span
      >
    </div>
    <div class="metric-row">
      <div><strong>{app.nodes.length}</strong><span>话术步骤</span></div>
      <div>
        <strong>{app.materials.length}</strong><span>知识库素材</span>
      </div>
      <div>
        <strong>{bytes(app.materials.reduce((n, m) => n + m.size, 0))}</strong
        ><span>素材总大小</span>
      </div>
    </div>
    <p class="field-help">
      发布前会检查流程连接。业务员已修改的话术保持个人版本，删除的素材在成功同步后移除。
    </p>
    {#if published}<small class="muted"
        >上次发布 v{published.version} · {published.nodes.length} 条话术 · {published
          .materials.length} 份素材</small
      >{/if}
  </div>
{:else}<div class="settings-card">
    <h3>管理员连接地址</h3>
    <label
      >同一工作组的管理员 IP
      <div class="inline-field">
        <input
          bind:value={address}
          placeholder="http://192.168.1.10:51876"
        /><button
          class="button secondary"
          disabled={!!app.busy}
          onclick={() =>
            app.task(
              '保存连接地址',
              async () => {
                await api('/group/address', 'PUT', { address });
                await app.refresh();
              },
              '管理员地址已更新',
            )}>保存</button
        >
      </div></label
    >
    <p class="field-help">
      只允许连接当前工作组；修改地址不会更换身份或清除个人话术。
    </p>
  </div>{/if}
