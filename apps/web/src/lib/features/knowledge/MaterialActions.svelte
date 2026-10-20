<script lang="ts">
  import { Copy, Download, ArrowUpRight } from 'lucide-svelte';
  import { app } from '../workspace/store.svelte';
  import { copyMaterial, exportMaterial } from '../workspace/api';
  import type { Material } from '$lib/types';
  let { material, ondetail }: { material: Material; ondetail?: () => void } =
    $props();
  let copyable = $derived(material.kind === 'image' || !material.hash);
  async function use() {
    if (copyable)
      await app.task(
        '复制素材',
        () => copyMaterial(material),
        material.kind === 'image' ? '已复制图片' : '已复制素材正文',
      );
    else await download();
  }
  async function download() {
    const exported = await app.task('导出素材', () => exportMaterial(material));
    if (exported) app.notify('已导出文件，请手动上传到云机');
  }
</script>

<div class="material-use-actions">
  <button
    type="button"
    class="button primary"
    disabled={!!app.busy || (!material.hash && !material.text)}
    aria-label={`${copyable ? '复制' : '导出'}${material.name}`}
    onclick={use}
  >
    {#if copyable}<Copy size={16} />{material.kind === 'image'
        ? '复制图片'
        : material.kind === 'case'
          ? '复制文案'
          : '复制正文'}{:else}<Download size={16} />导出文件{/if}
  </button>
  {#if material.kind === 'image' && material.hash}<button
      type="button"
      class="text-button"
      disabled={!!app.busy}
      aria-label={`导出${material.name}`}
      onclick={download}><Download size={15} />导出文件</button
    >{/if}
  {#if ondetail}<button type="button" class="text-button" onclick={ondetail}
      >详情<ArrowUpRight size={15} /></button
    >{/if}
</div>
