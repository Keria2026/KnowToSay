<script lang="ts">
  import { Search, Plus, X } from 'lucide-svelte';
  import { app } from '../workspace/store.svelte';
  import { searchLibrary } from '$lib/utils/scripts';
  import MaterialResult from './MaterialResult.svelte';
  let { ids = $bindable() }: { ids: string[] } = $props();
  let query = $state('');
  let limit = $state(20);
  let matches = $derived(
    query.trim()
      ? searchLibrary(app.materials, query).filter(
          (hit) => !ids.includes(hit.material.id),
        )
      : [],
  );
  $effect(() => {
    query;
    limit = 20;
  });
</script>

<div class="editor-section material-picker">
  <h3>关联素材</h3>
  <label class="material-link-search"
    ><Search size={17} /><input
      type="search"
      aria-label="搜索关联素材"
      bind:value={query}
      placeholder="搜索名称、标签或内容…"
    /></label
  >
  {#if query.trim()}<div class="material-link-matches" aria-label="可关联素材">
      {#each matches.slice(0, limit) as hit (hit.material.id)}
        <div class="material-link-match">
          <span>{hit.material.name}</span><button
            type="button"
            class="text-button"
            aria-label={`关联${hit.material.name}`}
            onclick={() => (ids = [...ids, hit.material.id])}
            ><Plus size={15} />关联</button
          >
        </div>
      {/each}
      {#if matches.length > limit}<button
          type="button"
          class="text-button"
          onclick={() => (limit += 20)}>显示更多素材</button
        >{/if}
      {#if !matches.length}<p class="field-help">没有可添加的匹配素材。</p>{/if}
    </div>{/if}
  <div class="linked-materials" aria-label="已关联素材">
    {#each ids as id (id)}{@const material = app.materials.find(
        (m) => m.id === id,
      )}
      <div class="linked-material">
        {#if material}<MaterialResult hit={{ material, rows: [] }} />{:else}<p
            class="field-help"
          >
            素材已移除
          </p>{/if}
        <button
          type="button"
          class="text-button"
          aria-label={`取消关联${material?.name ?? '已移除素材'}`}
          onclick={() => (ids = ids.filter((value) => value !== id))}
          ><X size={14} />取消关联</button
        >
      </div>
    {/each}
    {#if !ids.length}<p class="field-help">
        搜索并添加素材，关联后可预览、复制或导出。
      </p>{/if}
  </div>
</div>
