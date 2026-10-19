<script lang="ts">
  import {
    Download,
    Copy,
    FolderOpen,
    FileSpreadsheet,
    Play,
    FileText,
  } from 'lucide-svelte';
  import { app } from '../workspace/store.svelte';
  import {
    materialUrl,
    exportMaterial,
    revealMaterial,
    native,
    api,
  } from '../workspace/api';
  import { bytes, kindName } from '$lib/utils/scripts';
  import SheetPreview from './SheetPreview.svelte';
  import type { Material } from '$lib/types';
  let { material, query = '' }: { material: Material; query?: string } =
    $props();
  let url = $state('');
  let failed = $state(false);
  $effect(() => {
    const current = material;
    let active = true;
    failed = false;
    url = '';
    if (current.hash)
      materialUrl(current)
        .then((value) => {
          if (active) url = value;
        })
        .catch(() => {
          if (active) failed = true;
        });
    return () => {
      active = false;
    };
  });
</script>

<div class="preview-meta">
  <span class="pill">{kindName(material.kind)}</span><span
    >{material.category || '未分类'}</span
  >{#if material.hash}<span>{bytes(material.size)}</span>{/if}
</div>
{#if material.notes}<p class="muted">{material.notes}</p>{/if}
<div class="material-preview">
  {#if failed}<div class="empty small">
      此文件暂时无法预览，请导出后使用本机应用打开。
    </div>
  {:else if material.kind === 'image' && url}<img
      src={url}
      alt={material.name}
      onerror={() => (failed = true)}
    />
  {:else if material.kind === 'video' && url}
    <!-- svelte-ignore a11y_media_has_caption -->
    <video controls preload="metadata" src={url} onerror={() => (failed = true)}
      >浏览器不支持此格式，请导出后播放。</video
    >
  {:else if material.kind === 'text'}<div class="text-preview">
      {material.text || '暂无正文'}
    </div>
  {:else if material.kind === 'excel'}
    {#key material.id}<SheetPreview materialId={material.id} {query} />{/key}
  {:else}<div class="empty small">
      <FileText size={40} />
      <p>导出文件后在本机应用中打开。</p>
    </div>{/if}
</div>
{#if material.tags.length}<div class="tag-list">
    {#each material.tags as tag}<span class="tag">{tag}</span>{/each}
  </div>{/if}
<div class="button-row">
  <button
    class="button primary"
    disabled={!!app.busy}
    onclick={() =>
      app.task(
        '导出素材',
        () => exportMaterial(material),
        material.hash
          ? '已处理导出请求；请手动上传到 LDCLOUD'
          : '已复制素材正文',
      )}
    >{#if material.hash}<Download size={16} />导出文件{:else}<Copy
        size={16}
      />复制正文{/if}</button
  >
  {#if material.hash && native()}<button
      class="button secondary"
      onclick={() => app.task('定位素材', () => revealMaterial(material.id))}
      ><FolderOpen size={16} />打开所在位置</button
    >{/if}
</div>
{#if material.hash}<p class="field-help">
    导出文件保留原文件名，便于上传至 LDCLOUD 后在 KakaoTalk 选择发送。
  </p>{/if}
