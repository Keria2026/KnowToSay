<script lang="ts">
  import { onMount } from 'svelte';
  import { Download, Upload, Copy, Check, RefreshCw } from 'lucide-svelte';
  import { app } from './store.svelte';
  import { api, chooseDirectory, native } from './api';
  import type { FileResult, Snapshot, LanStatus } from '$lib/types';
  import SyncSettings from './SyncSettings.svelte';
  import DesktopSettings from './DesktopSettings.svelte';
  let exportDirectory = $state('');
  let importDirectory = $state('');
  let result = $state('');
  let network = $state<LanStatus | null>(null);
  let networkMessage = $state('');
  let checkingNetwork = $state(false);
  onMount(() => {
    void refreshNetwork();
    const timer = setInterval(() => void refreshNetwork(), 20_000);
    return () => clearInterval(timer);
  });
  async function refreshNetwork() {
    if (checkingNetwork) return;
    checkingNetwork = true;
    try {
      network = await api<LanStatus>('/group/network');
      networkMessage = '';
    } catch {
      networkMessage = '暂时无法获取网络信息，请重试。';
    } finally {
      checkingNetwork = false;
    }
  }
  async function exportAssets() {
    const directory = native() ? await chooseDirectory() : exportDirectory;
    if (native() && !directory) return;
    await app.task(
      '正在导出资产',
      async () => {
        result = (
          await api<FileResult>('/assets/export', 'POST', {
            directory: directory || null,
          })
        ).path;
      },
      '资产已导出，请保存整个文件夹',
    );
  }
  async function importAssets() {
    const directory = native() ? await chooseDirectory() : importDirectory;
    if (native() && !directory) return;
    if (!directory?.trim()) {
      app.error = '请选择资产文件夹';
      return;
    }
    if (
      !confirm(
        app.admin
          ? '导入资产到当前工作组主库？现有资产会保留，导入后点击“发布更新”即可同步给业务员。'
          : '导入资产？个人资产会替换当前个人分支；工作组资产只更新已发布资料并保留个人修改。',
      )
    )
      return;
    await app.task(
      '正在校验并导入资产',
      async () => {
        app.data = await api<Snapshot>('/assets/import', 'POST', { directory });
      },
      app.admin ? '资产已导入，检查后发布更新' : '资产已导入',
    );
  }
</script>

<section class="settings-page">
  <div class="page-heading">
    <div>
      <span class="eyebrow">YOUR LOCAL WORKSPACE</span>
      <h2>设置</h2>
      <p class="muted">管理工作组、资产和同步。</p>
    </div>
  </div>
  <div class="settings-card">
    <h3>当前工作组</h3>
    <dl class="workspace-details">
      <div>
        <dt>名称</dt>
        <dd>{app.workspace?.name}</dd>
      </div>
      <div>
        <dt>此电脑的角色</dt>
        <dd>{app.admin ? '管理员' : '业务员'}</dd>
      </div>
      {#if app.admin}
        <div>
          <dt>加入地址（IP 和端口）</dt>
          <dd>
            {#if network?.address}<code>{network.address}</code><button
                class="icon-button"
                aria-label="复制加入地址"
                onclick={() => app.copy(network?.address || '')}
                ><Copy size={16} /></button
              >
            {:else}<span
                >{checkingNetwork ? '正在获取…' : '未检测到局域网地址'}</span
              >{/if}
            <button
              class="icon-button"
              aria-label="刷新加入地址"
              disabled={checkingNetwork}
              onclick={refreshNetwork}><RefreshCw size={16} /></button
            >
          </dd>
        </div>
      {:else}
        <div>
          <dt>管理员地址</dt>
          <dd><code>{app.workspace?.adminAddress || '尚未设置'}</code></dd>
        </div>
      {/if}
      <div>
        <dt>已发布版本</dt>
        <dd>v{app.workspace?.version || 0}</dd>
      </div>
    </dl>
    {#if app.admin}<p class="field-help">
        {networkMessage ||
          (network && !network.available
            ? `局域网共享服务未启动，请检查端口 ${network.port} 是否被其他程序占用。`
            : '新人可自动发现工作组，也可以输入上面的 IP 和端口加入。请保持管理员客户端运行。')}{!app
          .workspace?.version
          ? ' 发布第一次资料后即可加入。'
          : ''}
      </p>{/if}
  </div>
  <div class="settings-card" id="workspace-assets">
    <h3>资产</h3>
    <p class="muted">
      {app.admin
        ? '导出话术、流程和完整知识库，用于保存、备份或交给其他管理员。导入后成为当前工作组的主库资产，发布后统一同步给业务员。'
        : '导出自己的话术分支、流程和分类。导入个人资产可恢复分支；导入本工作组的管理员资产可离线更新已发布资料。'}
    </p>
    {#if !native()}
      <label
        >导出位置（可留空）<input
          bind:value={exportDirectory}
          placeholder="保存资产的父文件夹绝对路径"
        /></label
      >
      <label
        >导入资产文件夹<input
          bind:value={importDirectory}
          placeholder="导出生成的完整资产文件夹绝对路径"
        /></label
      >
    {/if}
    <div class="asset-actions">
      <button
        class="button secondary"
        disabled={!!app.busy}
        onclick={exportAssets}><Download size={16} />导出资产</button
      >
      <button
        class="button primary"
        disabled={!!app.busy}
        onclick={importAssets}><Upload size={16} />导入资产</button
      >
    </div>
    {#if result}<div class="result-path">
        <Check size={16} />已导出：<code>{result}</code>
      </div>{/if}
    <p class="field-help">
      {app.admin
        ? '不包含其他电脑的个人分支。转移资产时请复制整个文件夹，包含所有素材。'
        : '个人资产仅供同一工作组的业务员使用，不包含统一管理的知识库。'}
    </p>
  </div>
  <SyncSettings />
  <DesktopSettings />
</section>
