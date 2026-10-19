<script lang="ts">
  import { MessageSquare } from 'lucide-svelte';
  import { app } from './store.svelte';
  import { api } from './api';
  import { searchScripts } from '$lib/utils/scripts';
  import { scopedScripts, scopedMaterials } from './search';
  import type { MaterialSearchResult } from '$lib/types';
  import ScriptResult from '../scripts/ScriptResult.svelte';
  import MaterialResult from '../knowledge/MaterialResult.svelte';
  let {
    query = app.query,
    dense = false,
    onclear = () => {
      app.query = '';
    },
  }: { query?: string; dense?: boolean; onclear?: () => void } = $props();
  let scripts = $derived(
    searchScripts(scopedScripts(app.nodes, app.searchScope), query),
  );
  let hits = $state<MaterialSearchResult[]>([]);
  let materials = $derived(scopedMaterials(hits, app.searchScope));
  let loading = $state(false);
  let error = $state('');
  let limit = $state(50);
  $effect(() => {
    const q = query;
    const scope = app.searchScope;
    let active = true;
    limit = 50;
    hits = [];
    error = '';
    if (!['all', 'library'].includes(scope.area)) {
      loading = false;
      return;
    }
    loading = true;
    const category =
      scope.area === 'library' && scope.group
        ? `&category=${encodeURIComponent(scope.group)}`
        : '';
    const timer = setTimeout(() => {
      api<MaterialSearchResult[]>(
        `/search/materials?q=${encodeURIComponent(q)}${category}`,
      )
        .then((result) => {
          if (active) hits = result;
        })
        .catch((e) => {
          if (active) error = String(e);
        })
        .finally(() => {
          if (active) loading = false;
        });
    }, 120);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  });
</script>

<section class="search-page" class:dense>
  {#if !dense}<div class="page-heading">
      <div>
        <span class="eyebrow">搜索结果</span>
        <h2>找到合适的回答</h2>
        <p class="muted">{scripts.length} 条话术 · {materials.length} 份素材</p>
      </div>
      <button class="button secondary" onclick={onclear}>返回浏览</button>
    </div>{:else}<p class="field-help">
      {scripts.length} 条话术 · {materials.length} 份素材
    </p>{/if}
  {#if scripts.length}<div class="section-label">话术 · 可直接复制</div>
    <div class="search-list">
      {#each scripts.slice(0, limit) as record (record.node.id)}<ScriptResult
          {record}
        />{/each}
    </div>{/if}
  {#if scripts.length > limit}<button
      class="text-button"
      onclick={() => (limit += 50)}>显示更多话术</button
    >{/if}
  {#if materials.length}<div class="section-label">知识库与表格内容</div>
    <div class="search-list">
      {#each materials as hit (hit.material.id)}<MaterialResult
          {hit}
          {query}
        />{/each}
    </div>{/if}
  {#if loading}<p class="field-help" role="status">
      正在搜索本地知识库…
    </p>{:else if error}<p class="info-note" role="alert">
      {error}
    </p>{:else if !scripts.length && !materials.length}<div class="empty small">
      <MessageSquare size={32} />
      <h3>没有找到匹配内容</h3>
      <p>试试问题关键词，或切换搜索范围。</p>
    </div>{/if}
</section>
