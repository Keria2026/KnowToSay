<script lang="ts">
  import { MessageSquare } from 'lucide-svelte';
  import { app } from './store.svelte';
  import { SearchResultsModel } from './search-results.svelte';
  import ScriptResult from '../scripts/ScriptResult.svelte';
  import MaterialResult from '../knowledge/MaterialResult.svelte';
  let {
    query = app.query,
    dense = false,
    onclear = () => {
      app.query = '';
    },
  }: { query?: string; dense?: boolean; onclear?: () => void } = $props();
  const results = new SearchResultsModel(() => query);
</script>

<section class="search-page" class:dense>
  {#if !dense}<div class="page-heading">
      <div>
        <span class="eyebrow">搜索结果</span>
        <h2>找到合适的回答</h2>
        <p class="muted">
          {results.scripts.length} 条话术 · {results.materials.length} 份素材
        </p>
      </div>
      <button class="button secondary" onclick={onclear}>返回浏览</button>
    </div>{:else}<p class="field-help">
      {results.scripts.length} 条话术 · {results.materials.length} 份素材
    </p>{/if}
  {#if results.scripts.length}<div class="section-label">话术 · 可直接复制</div>
    <div class="search-list">
      {#each results.visibleScripts as record (record.node.id)}<ScriptResult
          {record}
          {query}
        />{/each}
    </div>{/if}
  {#if results.scripts.length > results.limit}<button
      class="text-button"
      onclick={() => (results.limit += 50)}>显示更多话术</button
    >{/if}
  {#if results.materials.length}<div class="section-label">
      知识库与表格内容
    </div>
    <div class="search-list">
      {#each results.materials as hit (hit.material.id)}<MaterialResult
          {hit}
          {query}
        />{/each}
    </div>{/if}
  {#if results.loading}<p class="field-help" role="status">
      正在搜索本地知识库…
    </p>{:else if results.error}<p class="info-note" role="alert">
      {results.error}
    </p>{:else if !results.scripts.length && !results.materials.length}<div
      class="empty small"
    >
      <MessageSquare size={32} />
      <h3>没有找到匹配内容</h3>
      <p>试试问题关键词，或切换搜索范围。</p>
    </div>{/if}
</section>
