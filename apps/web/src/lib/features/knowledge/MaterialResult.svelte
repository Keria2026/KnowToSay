<script lang="ts">
  import { Copy, Download, FileText, ArrowUpRight } from 'lucide-svelte';
  import { app } from '../workspace/store.svelte';
  import { exportMaterial } from '../workspace/api';
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
  async function useMaterial() {
    if (!hit.material.hash) {
      await app.copy(hit.material.text);
      return;
    }
    const exported = await app.task('导出素材', () =>
      exportMaterial(hit.material),
    );
    if (exported) app.notify('已处理文件导出，请手动上传到云机');
  }
</script>

<article class="search-result result-card" class:active>
  <button
    class="result-summary"
    aria-label={`查看${hit.material.name}详情`}
    onclick={detail}
    ><FileText size={18} /><span
      ><strong>{hit.material.name}</strong><small
        >{hit.material.notes ||
          hit.material.text ||
          hit.material.category}</small
      >{#each hit.rows.slice(0, 3) as row}<span class="excel-hit"
          >{row.sheet} · 第 {row.row} 行：{row.cells.join(' ｜ ')}</span
        >{/each}<span class="tag">{kindName(hit.material.kind)}</span></span
    ></button
  >
  <div class="result-actions">
    <button
      class="button primary"
      disabled={!!app.busy || (!hit.material.hash && !hit.material.text)}
      aria-label={`${hit.material.hash ? '导出' : '复制'}${hit.material.name}`}
      onclick={useMaterial}
      >{#if hit.material.hash}<Download size={16} />{:else}<Copy
          size={16}
        />{/if}{hit.material.hash ? '导出文件' : '复制正文'}</button
    ><button class="text-button" onclick={detail}
      >详情<ArrowUpRight size={15} /></button
    >
  </div>
</article>
