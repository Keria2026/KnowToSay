<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { Search, X, ArrowUpRight, Copy, ArrowLeft } from 'lucide-svelte';
  import { app } from './store.svelte';
  import { native, copyText } from './api';
  import {
    replyPreview,
    languageName,
    scriptKindName,
    replyVariables,
  } from '$lib/utils/scripts';
  import type {
    NodeRecord,
    DesktopStatus,
    SearchLaunchScope,
  } from '$lib/types';
  import './quick-search.css';
  import SearchScopePicker from './SearchScopePicker.svelte';
  import { SearchResultsModel } from './search-results.svelte';
  import ScriptResult from '../scripts/ScriptResult.svelte';
  import MaterialResult from '../knowledge/MaterialResult.svelte';
  import BilingualReplies from '../scripts/BilingualReplies.svelte';
  let query = $state('');
  let selected = $state<NodeRecord | null>(null);
  let values = $state<Record<string, string>>({});
  let activeIndex = $state(0);
  let input: HTMLInputElement;
  let copyHides = true;
  const results = new SearchResultsModel(() => query);
  let preview = $derived(
    selected ? replyPreview(selected.node, app.language, values) : null,
  );
  let variables = $derived(
    selected ? replyVariables(selected.node, app.language) : [],
  );
  $effect(() => {
    app.searchScope;
    selected = null;
    activeIndex = 0;
  });
  $effect(() => {
    if (native())
      void invoke('resize_search', {
        expanded:
          app.searchScope.area === 'quick' || !!query.trim() || !!selected,
      }).catch((e) => (app.error = String(e)));
  });
  onMount(() => {
    app.searchScope = {
      area:
        new URLSearchParams(window.location.search).get('scope') === 'quick'
          ? 'quick'
          : 'all',
      group: '',
    };
    void tick().then(() => input?.focus());
    if (!native()) return;
    const setup = listen<SearchLaunchScope>('search-open', async (event) => {
      app.searchScope = { area: event.payload, group: '' };
      query = '';
      selected = null;
      values = {};
      activeIndex = 0;
      await app.refresh();
      try {
        copyHides = (await invoke<DesktopStatus>('desktop_status')).preferences
          .copyHides;
      } catch (e) {
        app.error = String(e);
      }
      await tick();
      input?.focus();
    });
    void invoke<DesktopStatus>('desktop_status')
      .then((status) => {
        copyHides = status.preferences.copyHides;
        app.searchScope = { area: status.searchLaunchScope, group: '' };
      })
      .catch((e) => (app.error = String(e)));
    return () => {
      void setup.then((stop) => stop());
    };
  });
  async function close() {
    if (native()) await invoke('close_search');
    else window.close();
  }
  function select(record: NodeRecord) {
    selected = record;
    values = {};
  }
  async function openMain(id = '') {
    if (native()) await invoke('open_script', { id });
    else window.location.href = `/?script=${encodeURIComponent(id)}`;
  }
  async function copy() {
    if (!preview?.ready) return;
    await copyReply(preview.text);
  }
  async function copyReply(text: string) {
    const copied = await app.task('复制回复', async () => {
      await copyText(text);
      return true;
    });
    if (copied) {
      if (native() && copyHides) await close();
      else app.notify('已复制回复');
    }
  }
  function keyboard(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      void close();
    }
    if (event.target !== input || selected) return;
    const count = results.visibleScripts.length + results.materials.length;
    if (event.key === 'ArrowDown' && count) {
      event.preventDefault();
      activeIndex = (activeIndex + 1) % count;
    }
    if (event.key === 'ArrowUp' && count) {
      event.preventDefault();
      activeIndex = (activeIndex + count - 1) % count;
    }
    if (event.key === 'Enter' && count) {
      event.preventDefault();
      if (activeIndex < results.visibleScripts.length) {
        const record = results.visibleScripts[activeIndex];
        const ready = replyPreview(record.node, app.language, {});
        if (ready.ready) void copyReply(ready.text);
        else select(record);
      } else {
        const hit =
          results.materials[activeIndex - results.visibleScripts.length];
        app.materialQuery = hit.rows.length ? query : '';
        app.selectedMaterialId = hit.material.id;
      }
    }
  }
</script>

<svelte:window onkeydown={keyboard} />
<div class="quick-search-shell">
  <header class="quick-search-input">
    <span class="quick-search-mark" data-tauri-drag-region
      ><Search size={24} strokeWidth={1.6} /></span
    ><input
      bind:this={input}
      bind:value={query}
      oninput={() => {
        selected = null;
        activeIndex = 0;
      }}
      aria-label="独立搜索栏"
      placeholder={app.searchScope.area === 'quick'
        ? '搜索客户问题或快捷回复…'
        : '搜索话术、素材、表格内容…'}
      autocomplete="off"
    /><select bind:value={app.language} aria-label="独立搜索发送语言"
      >{#each app.languages as language}<option value={language}
          >{languageName(language)}</option
        >{/each}</select
    ><button class="icon-button" aria-label="收起搜索栏" onclick={close}
      ><X size={18} /></button
    >
  </header>
  <SearchScopePicker bind:scope={app.searchScope} />
  {#if selected}
    <section class="quick-search-detail">
      <div class="section-title">
        <button class="text-button" onclick={() => (selected = null)}
          ><ArrowLeft size={14} />返回结果</button
        ><button class="text-button" onclick={() => openMain(selected!.node.id)}
          >在工作台打开<ArrowUpRight size={14} /></button
        >
      </div>
      <h2>{selected.node.title}</h2>
      <p class="muted">
        {scriptKindName(selected.node.kind)} · {selected.node.question}
      </p>
      {#if variables.length}<div class="quick-variables">
          {#each variables as variable}<label
              >{variable.label || variable.name}<input
                bind:value={values[variable.name]}
                placeholder={`请输入${variable.label || variable.name}`}
              /></label
            >{/each}
        </div>{/if}
      <BilingualReplies node={selected.node} />
      <p class="field-help">复制预览 · {languageName(app.language)}</p>
      <p class="text-preview">
        {preview?.languageMissing
          ? '暂无此语言版本，请选择其他语言。'
          : preview?.text}
      </p>
      <div class="reply-actions">
        <span class="field-help"
          >{preview?.missing.length
            ? `待填写：${preview.missing.join('、')}`
            : selected.source === 'personal'
              ? '个人版本'
              : '管理员版本'}</span
        ><button
          class="button primary"
          disabled={!preview?.ready || !!app.busy}
          onclick={copy}><Copy size={16} />复制回复</button
        >
      </div>
    </section>
  {:else if !query.trim() && app.searchScope.area !== 'quick'}
    <p class="quick-search-hint">输入关键词搜索，或选择快捷话术直接浏览。</p>
  {:else}
    <div class="quick-search-results">
      {#if results.visibleScripts.length}<div class="section-label">
          话术 · 可直接复制
        </div>{/if}
      {#each results.visibleScripts as record, i (record.node.id)}<ScriptResult
          {record}
          active={activeIndex === i}
          oncopy={copyReply}
          ondetail={() => select(record)}
        />{/each}
      {#if results.materials.length}<div class="section-label">
          素材与表格
        </div>{/if}
      {#each results.materials as hit, i (hit.material.id)}<MaterialResult
          {hit}
          {query}
          active={activeIndex === results.visibleScripts.length + i}
        />{/each}
      {#if results.loading}<p class="field-help">
          正在搜索本地知识库…
        </p>{:else if results.error}<p class="info-note" role="alert">
          {results.error}
        </p>{:else if !results.visibleScripts.length && !results.materials.length}<div
          class="empty small"
        >
          <Search size={30} />
          <p>没有匹配内容，试试问题关键词或产品型号。</p>
        </div>{/if}
      {#if results.scripts.length > results.limit}<button
          class="text-button"
          onclick={() => (results.limit += 50)}
          >显示更多话术（已显示 {results.visibleScripts.length} / {results
            .scripts.length}）</button
        >{/if}
    </div>
    <footer class="quick-search-hint">
      <span
        >{results.scripts.length} 条话术 · {results.materials.length} 份素材 · Esc
        收起</span
      ><button class="text-button" onclick={() => openMain()}
        >打开完整工作台<ArrowUpRight size={13} /></button
      >
    </footer>
  {/if}
</div>
