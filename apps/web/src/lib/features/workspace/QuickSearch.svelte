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
  } from '$lib/utils/scripts';
  import type {
    NodeRecord,
    MaterialSearchResult,
    DesktopStatus,
  } from '$lib/types';
  import './quick-search.css';
  let query = $state('');
  let selected = $state<NodeRecord | null>(null);
  let values = $state<Record<string, string>>({});
  let materials = $state<MaterialSearchResult[]>([]);
  let searching = $state(false);
  let activeIndex = $state(0);
  let input: HTMLInputElement;
  let copyHides = true;
  let scripts = $derived(
    query.trim() ? searchScripts(app.nodes, query).slice(0, 8) : [],
  );
  let preview = $derived(
    selected ? replyPreview(selected.node, app.language, values) : null,
  );
  let variables = $derived(
    selected
      ? [
          ...new Map(
            [
              ...selected.node.variables,
              ...[
                ...(selected.node.replies[app.language] || '').matchAll(
                  /\{([^{}]+)\}/g,
                ),
              ].map(
                (m) =>
                  selected!.node.variables.find((v) => v.name === m[1]) || {
                    name: m[1],
                    label: m[1],
                    required: true,
                  },
              ),
            ].map((v) => [v.name, v]),
          ).values(),
        ]
      : [],
  );
  $effect(() => {
    const q = query.trim();
    let active = true;
    if (!q || !app.workspace) {
      materials = [];
      searching = false;
      return;
    }
    searching = true;
    const timer = setTimeout(() => {
      api<MaterialSearchResult[]>(
        `/search/materials?q=${encodeURIComponent(q)}`,
      )
        .then((result) => {
          if (active) materials = result.slice(0, 8);
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
        expanded: !!query.trim() || !!selected,
      }).catch((e) => (app.error = String(e)));
  });
  onMount(() => {
    void tick().then(() => input?.focus());
    if (!native()) return;
    const setup = listen('quick-search-open', async () => {
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
    await app.task('复制回复', () => copyText(preview!.text));
    if (!app.error) {
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
      if (activeIndex < scripts.length) select(scripts[activeIndex]);
      else
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
      placeholder="搜索话术、素材、表格内容…"
      autocomplete="off"
    /><select bind:value={app.language} aria-label="独立搜索发送语言"
      >{#each app.languages as language}<option value={language}
          >{languageName(language)}</option
        >{/each}</select
    ><button class="icon-button" aria-label="收起搜索栏" onclick={close}
      ><X size={18} /></button
    >
  </header>
  {#if !query.trim() && !selected}<footer class="quick-search-hint">
      <span>{app.workspace ? app.workspace.name : '请先创建或加入工作组'}</span
      ><span>↑ ↓ 选择 · Enter 查看 · Esc 收起</span>
    </footer>
  {:else if selected}
    <section class="quick-search-detail">
      <div class="section-title">
        <button class="text-button" onclick={() => (selected = null)}
          ><ArrowLeft size={14} />返回结果</button
        ><button class="text-button" onclick={() => openMain(selected!.node.id)}
          >在工作台打开<ArrowUpRight size={14} /></button
        >
      </div>
      <h2>{selected.node.title}</h2>
      <p class="muted">{selected.node.question}</p>
      {#if variables.length}<div class="quick-variables">
          {#each variables as variable}<label
              >{variable.label || variable.name}<input
                bind:value={values[variable.name]}
                placeholder={`请输入${variable.label || variable.name}`}
              /></label
            >{/each}
        </div>{/if}
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
  {:else}
    <div class="quick-search-results">
      {#if scripts.length}<div class="section-label">
          话术
        </div>{/if}{#each scripts as record, i}<button
          class:active={activeIndex === i}
          class="quick-result"
          onclick={() => select(record)}
          ><MessageSquare size={18} /><span
            ><strong>{record.node.title}</strong><small
              >{record.node.replies[app.language] ||
                record.node.question}</small
            ></span
          ><span class="source-label"
            >{record.source === 'personal' ? '个人' : '官方'}</span
          ></button
        >{/each}{#if materials.length}<div class="section-label">
          素材与表格
        </div>{/if}{#each materials as hit, i}<button
          class:active={activeIndex === scripts.length + i}
          class="quick-result"
          onclick={() => {
            app.query = query;
            app.selectedMaterialId = hit.material.id;
          }}
          ><FileText size={18} /><span
            ><strong>{hit.material.name}</strong><small
              >{hit.rows.length
                ? `${hit.rows[0].sheet} · 第 ${hit.rows[0].row} 行：${hit.rows[0].cells.join(' ｜ ')}`
                : hit.material.notes || hit.material.category}</small
            ></span
          ><span class="source-label">{kindName(hit.material.kind)}</span
          ></button
        >{/each}{#if searching}<p class="field-help">
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
        >{scripts.length + materials.length} 个结果（最多展示各类 8 项）</span
      ><button class="text-button" onclick={() => openMain()}
        >打开完整工作台<ArrowUpRight size={13} /></button
      >
    </footer>
  {/if}
</div>
