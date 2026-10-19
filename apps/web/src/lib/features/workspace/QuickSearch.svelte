<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import {
    Search,
    X,
    ArrowUpRight,
    Copy,
    MessageSquare,
    FileText,
    ArrowLeft,
  } from 'lucide-svelte';
  import { app } from './store.svelte';
  import { api, native, copyText } from './api';
  import {
    searchScripts,
    replyPreview,
    languageName,
    kindName,
    scriptKindName,
    replyVariables,
    quickCategoryName,
  } from '$lib/utils/scripts';
  import type {
    NodeRecord,
    MaterialSearchResult,
    DesktopStatus,
    SearchPanelMode,
  } from '$lib/types';
  import './quick-search.css';
  import SearchScopePicker from './SearchScopePicker.svelte';
  import { scopedScripts, scopedMaterials, globalScope } from './search';
  import ScriptResult from '../scripts/ScriptResult.svelte';
  import MaterialResult from '../knowledge/MaterialResult.svelte';
  import BilingualReplies from '../scripts/BilingualReplies.svelte';
  let query = $state('');
  let mode = $state<SearchPanelMode>('search');
  let visibleLimit = $state(50);
  let selected = $state<NodeRecord | null>(null);
  let values = $state<Record<string, string>>({});
  let materials = $state<MaterialSearchResult[]>([]);
  let searching = $state(false);
  let activeIndex = $state(0);
  let input: HTMLInputElement;
  let copyHides = true;
  let quickScripts = $derived(
    searchScripts(
      app.nodes.filter(
        (r) =>
          r.node.kind === 'quick' &&
          (!app.quickCategory ||
            quickCategoryName(r.node) === app.quickCategory),
      ),
      query,
    ),
  );
  let scripts = $derived(
    mode === 'quick'
      ? quickScripts.slice(0, visibleLimit)
      : query.trim()
        ? searchScripts(scopedScripts(app.nodes, app.searchScope), query).slice(
            0,
            50,
          )
        : [],
  );
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
    const q = query.trim();
    const scope = app.searchScope;
    let active = true;
    if (
      mode === 'quick' ||
      !q ||
      !app.workspace ||
      !['all', 'library'].includes(scope.area)
    ) {
      materials = [];
      searching = false;
      return;
    }
    searching = true;
    const timer = setTimeout(() => {
      api<MaterialSearchResult[]>(
        `/search/materials?q=${encodeURIComponent(q)}${scope.area === 'library' && scope.group ? `&category=${encodeURIComponent(scope.group)}` : ''}`,
      )
        .then((result) => {
          if (active) materials = scopedMaterials(result, scope).slice(0, 50);
        })
        .catch((e) => {
          if (active) app.error = String(e);
        })
        .finally(() => {
          if (active) searching = false;
        });
    }, 120);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  });
  $effect(() => {
    if (native())
      void invoke('resize_search', {
        expanded: mode === 'quick' || !!query.trim() || !!selected,
      }).catch((e) => (app.error = String(e)));
  });
  onMount(() => {
    if (new URLSearchParams(window.location.search).get('panel') === 'quick')
      mode = 'quick';
    void tick().then(() => input?.focus());
    if (!native()) return;
    const setup = listen<SearchPanelMode>(
      'quick-search-open',
      async (event) => {
        mode = event.payload;
        app.searchScope = globalScope();
        app.quickCategory = '';
        visibleLimit = 50;
        query = '';
        selected = null;
        values = {};
        activeIndex = 0;
        await app.refresh();
        try {
          copyHides = (await invoke<DesktopStatus>('desktop_status'))
            .preferences.copyHides;
        } catch (e) {
          app.error = String(e);
        }
        await tick();
        input?.focus();
      },
    );
    void invoke<DesktopStatus>('desktop_status')
      .then((status) => {
        copyHides = status.preferences.copyHides;
        mode = status.searchMode;
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
    await app.task('复制回复', () => copyText(text));
    if (!app.error) {
      if (native() && copyHides) await close();
      else app.notify('已复制回复');
    }
  }
  function switchMode(next: SearchPanelMode) {
    if (native()) {
      void invoke(
        next === 'quick' ? 'open_quick_replies' : 'open_search',
      ).catch((e) => (app.error = String(e)));
    } else {
      mode = next;
      app.searchScope = globalScope();
      app.quickCategory = '';
      query = '';
      selected = null;
      visibleLimit = 50;
      activeIndex = 0;
    }
  }
  function keyboard(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      void close();
    }
    if (event.target !== input || selected) return;
    const count = scripts.length + materials.length;
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
      if (activeIndex < scripts.length) {
        const record = scripts[activeIndex];
        const ready = replyPreview(record.node, app.language, {});
        if (ready.ready) void copyReply(ready.text);
        else select(record);
      } else
        app.selectedMaterialId =
          materials[activeIndex - scripts.length].material.id;
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
      placeholder={mode === 'quick'
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
  {#if mode === 'search'}<SearchScopePicker bind:scope={app.searchScope} />{/if}
  {#if selected}
    <section class="quick-search-detail">
      <div class="section-title">
        <button class="text-button" onclick={() => (selected = null)}
          ><ArrowLeft size={14} />返回{mode === 'quick'
            ? '回复表'
            : '结果'}</button
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
  {:else if mode === 'quick'}
    <div class="quick-reply-heading">
      <div>
        <h2>快捷话术回复表</h2>
        <p class="field-help">
          {quickScripts.length} 条问答 · {languageName(app.language)}
        </p>
      </div>
      <button class="text-button" onclick={() => switchMode('search')}
        >全库搜索<ArrowUpRight size={14} /></button
      >
    </div>
    <div class="quick-reply-list">
      <select aria-label="快捷回复分类" bind:value={app.quickCategory}
        ><option value="">全部分类</option
        >{#each app.quickGroups as group}<option value={group.id}
            >{group.name}</option
          >{/each}</select
      >
      {#each scripts as record, i (record.node.id)}<ScriptResult
          {record}
          active={activeIndex === i}
          oncopy={copyReply}
          ondetail={() => select(record)}
        />{/each}
      {#if !scripts.length}<div class="empty small">
          <p>没有匹配的快捷话术。</p>
        </div>{/if}
      {#if quickScripts.length > visibleLimit}<button
          class="text-button"
          onclick={() => (visibleLimit += 50)}
          >显示更多（已显示 {scripts.length} / {quickScripts.length}）</button
        >{/if}
    </div>
    <footer class="quick-search-hint">
      <span>选择后复制 · Esc 收起</span><button
        class="text-button"
        onclick={() => openMain()}
        >打开完整工作台<ArrowUpRight size={13} /></button
      >
    </footer>
  {:else if !query.trim()}<footer class="quick-search-hint">
      <span>{app.workspace ? app.workspace.name : '请先创建或加入工作组'}</span
      ><button class="text-button" onclick={() => switchMode('quick')}
        >快捷话术回复表</button
      >
    </footer>
  {:else}
    <div class="quick-search-results">
      {#if scripts.length}<div class="section-label">
          话术 · 可直接复制
        </div>{/if}
      {#each scripts as record, i (record.node.id)}<ScriptResult
          {record}
          active={activeIndex === i}
          oncopy={copyReply}
          ondetail={() => select(record)}
        />{/each}
      {#if materials.length}<div class="section-label">素材与表格</div>{/if}
      {#each materials as hit, i (hit.material.id)}<MaterialResult
          {hit}
          {query}
          active={activeIndex === scripts.length + i}
        />{/each}
      {#if searching}<p class="field-help">
          正在搜索本地知识库…
        </p>{:else if !scripts.length && !materials.length}<div
          class="empty small"
        >
          <Search size={30} />
          <p>没有匹配内容，试试问题关键词或产品型号。</p>
        </div>{/if}
    </div>
    <footer class="quick-search-hint">
      <span
        >{scripts.length + materials.length} 个结果（最多展示各类 50 项）</span
      ><button class="text-button" onclick={() => openMain()}
        >打开完整工作台<ArrowUpRight size={13} /></button
      >
    </footer>
  {/if}
</div>
