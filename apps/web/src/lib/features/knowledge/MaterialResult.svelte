<script lang="ts">
  import { FileText } from 'lucide-svelte';
  import { app } from '../workspace/store.svelte';
  import MaterialActions from './MaterialActions.svelte';
  import MaterialImage from './MaterialImage.svelte';
  import { folderPath } from './folders';
  import { kindName } from '$lib/utils/scripts';
  import type { MaterialSearchResult } from '$lib/types';
  let {
    hit,
    query = '',
    active = false,
  }: { hit: MaterialSearchResult; query?: string; active?: boolean } = $props();
  function detail() {
    app.materialQuery = hit.rows.length ? query : '';
    app.selectedMaterialId = hit.material.id;
  }
</script>

<article class="search-result result-card" class:active>
  <button
    type="button"
    class="result-summary"
    aria-label={`查看${hit.material.name}详情`}
    onclick={detail}
    >{#if hit.material.kind === 'image'}<MaterialImage
        material={hit.material}
      />{:else}<FileText size={18} />{/if}<span
      ><strong>{hit.material.name}</strong><small
        >{hit.material.text.slice(0, 90) ||
          folderPath(app.folders, hit.material.folderId)}</small
      >{#each hit.rows.slice(0, 3) as row}<span class="excel-hit"
          >{row.sheet} · 第 {row.row} 行：{row.cells.join(' ｜ ')}</span
        >{/each}<span class="tag">{kindName(hit.material.kind)}</span></span
    ></button
  >
  <div class="result-actions">
    <MaterialActions material={hit.material} ondetail={detail} />
  </div>
</article>
