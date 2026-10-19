<script lang="ts">
  import { ArrowRight, Radio, Plus, Search, FolderOpen } from 'lucide-svelte';
  import { app } from './store.svelte';
  import { api, chooseDirectory, native } from './api';
  import type { Discovery, Snapshot } from '$lib/types';
  import { bytes } from '$lib/utils/scripts';
  let tab = $state<'create' | 'join'>('join');
  let name = $state('销售工作组');
  let address = $state('');
  let groups = $state<Discovery[]>([]);
  let packagePath = $state('');
  async function submit() {
    await app.task(tab === 'create' ? '创建工作组' : '加入工作组', async () => {
      await api(
        tab === 'create' ? '/group' : '/group/join',
        'POST',
        tab === 'create' ? { name } : { address },
      );
      await app.refresh();
    });
  }
  async function find() {
    await app.task('搜索局域网', async () => {
      groups = await api<Discovery[]>('/group/discover');
      if (!groups.length)
        app.notify('未发现工作组，可手动填写管理员 IP；请确认管理员客户端在线');
    });
  }
  async function importPackage() {
    const directory = native() ? await chooseDirectory() : packagePath;
    if (!directory) return;
    await app.task('导入资料包', async () => {
      app.data = await api<Snapshot>('/package/import', 'POST', { directory });
    });
  }
  async function restoreAdmin() {
    const directory = native() ? await chooseDirectory() : packagePath;
    if (
      !directory ||
      !confirm(
        '恢复前请确保原管理员客户端已退出，避免同时运行两个管理员。继续恢复原工作组？',
      )
    )
      return;
    await app.task('校验并恢复管理员备份', async () => {
      app.data = await api<Snapshot>('/restore', 'POST', { directory });
    });
  }
</script>

<div class="onboarding">
  <div class="welcome-art">
    <img class="brand-mark large" src="/icon.svg" alt="KnowToSay" />
    <p class="eyebrow">KNOWLEDGE INTO CONVERSATION</p>
    <h1>把知识，<br />变成恰好的回答。</h1>
    <p class="welcome-sub">
      话术有路径，资料有归处。<br />与你的团队一起，让每次沟通更从容。
    </p>
    <div class="welcome-detail">
      <span class="status-dot"></span> 本地存储 · 离线可用 · 局域网同步
    </div>
  </div>
  <section class="setup-card">
    <span class="eyebrow">开始使用 KNOWTOSAY</span>
    <h2>连接你的工作组</h2>
    <p class="muted">团队第一台电脑创建工作组，其他电脑加入。</p>
    <div class="segmented">
      <button class:active={tab === 'join'} onclick={() => (tab = 'join')}
        >加入工作组</button
      ><button class:active={tab === 'create'} onclick={() => (tab = 'create')}
        >创建工作组</button
      >
    </div>
    <form
      onsubmit={(event) => {
        event.preventDefault();
        void submit();
      }}
    >
      {#if tab === 'create'}
        <label
          >工作组名称<input
            bind:value={name}
            required
            maxlength="80"
            placeholder="例如：韩国销售组"
          /></label
        >
        <div class="info-note">
          <Plus size={18} />
          <p>
            创建后，这台电脑成为管理员。你可以维护资料、发布更新和管理备份。
          </p>
        </div>
      {:else}
        <button
          class="button secondary full"
          type="button"
          onclick={find}
          disabled={!!app.busy}><Radio size={17} />发现局域网工作组</button
        >
        {#each groups as group}<button
            type="button"
            class="discovered"
            onclick={() => (address = group.address)}
            ><Radio size={18} /><span
              >{group.name}<small>{group.address}</small></span
            ><ArrowRight size={16} /></button
          >{/each}
        <label
          >管理员地址<input
            bind:value={address}
            required
            placeholder="192.168.1.10:51876"
          /></label
        >
        <p class="field-help">
          首次加入需要管理员在线且已发布资料。找不到管理员时不会自动创建新工作组。
        </p>
      {/if}
      <button class="button primary full" disabled={!!app.busy}
        >{app.busy ||
          (tab === 'create' ? '创建并进入工作台' : '加入并同步资料')}<ArrowRight
          size={17}
        /></button
      >
      {#if ['checking', 'downloading', 'applying'].includes(app.progress?.phase || '')}<p
          class="field-help"
          role="status"
        >
          {app.progress?.message}
        </p>
        {#if app.progress?.totalBytes}<progress
            value={app.progress.completedBytes}
            max={app.progress.totalBytes}
          ></progress>
          <p class="field-help">
            {bytes(app.progress.completedBytes)} / {bytes(
              app.progress.totalBytes,
            )}
          </p>{/if}{/if}
    </form>
    <details>
      <summary><FolderOpen size={15} />通过资料包加入</summary>
      <p class="field-help">选择管理员导出的资料包文件夹。</p>
      {#if !native()}<input
          bind:value={packagePath}
          placeholder="本机资料包文件夹绝对路径"
        />{/if}<button
        class="button secondary full"
        onclick={importPackage}
        disabled={!!app.busy}>导入资料包</button
      >
    </details>
    <details>
      <summary><FolderOpen size={15} />从管理员备份恢复</summary>
      <p class="field-help">
        管理员迁移或资料丢失后恢复原工作组。恢复前请关闭原管理员客户端。
      </p>
      {#if !native()}<input
          bind:value={packagePath}
          placeholder="包含 backup.json 的备份文件夹绝对路径"
        />{/if}<button
        class="button secondary full"
        onclick={restoreAdmin}
        disabled={!!app.busy}>选择并恢复管理员备份</button
      >
    </details>
  </section>
</div>
