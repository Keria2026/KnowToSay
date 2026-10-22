<script lang="ts">
  import { onMount } from 'svelte';
  import { ArrowRight, Radio, Plus } from 'lucide-svelte';
  import { app } from './store.svelte';
  import { api } from './api';
  import type { Discovery } from '$lib/types';
  import { bytes } from '$lib/utils/scripts';
  let tab = $state<'create' | 'join'>('join');
  let name = $state('销售工作组');
  let address = $state('');
  let groups = $state<Discovery[]>([]);
  let discovering = $state(false);
  let discoveryMessage = $state('');
  onMount(() => {
    void find();
  });
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
    if (discovering) return;
    discovering = true;
    discoveryMessage = '';
    try {
      groups = await api<Discovery[]>('/group/discover');
      if (!groups.length)
        discoveryMessage =
          '未发现工作组，请确认管理员在线，或输入 IP 和端口加入。';
    } catch {
      discoveryMessage = '自动发现暂不可用，仍可输入管理员的 IP 和端口加入。';
    } finally {
      discovering = false;
    }
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
    <p class="muted">加入已有团队，或创建自己的工作组。</p>
    <div class="segmented">
      <button class:active={tab === 'join'} onclick={() => (tab = 'join')}
        >发现并加入工作组</button
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
            创建后，这台电脑成为管理员。你可以在设置中导入资产，也可以直接开始整理话术和知识库。
          </p>
        </div>
      {:else}
        <button
          class="button secondary full"
          type="button"
          onclick={find}
          disabled={discovering || !!app.busy}
          ><Radio size={17} />{discovering
            ? '正在发现工作组…'
            : '重新发现'}</button
        >
        {#if discoveryMessage}<p class="field-help" role="status">
            {discoveryMessage}
          </p>{/if}
        {#each groups as group}<button
            type="button"
            class="discovered"
            disabled={!!app.busy}
            onclick={() => {
              address = group.address;
              void submit();
            }}
            ><Radio size={18} /><span
              >{group.name}<small>{group.address}</small></span
            ><ArrowRight size={16} /></button
          >{/each}
        <label
          >管理员 IP 和端口<input
            bind:value={address}
            required
            placeholder="192.168.1.10:51876"
          /></label
        >
        <p class="field-help">
          以业务员身份加入，自动接收管理员发布的资料。首次加入需要管理员在线且已发布资料。
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
  </section>
</div>
