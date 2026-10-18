<script lang="ts">
  import {
    ArrowUpRight,
    FileText,
    MessageSquare,
    FileSpreadsheet,
  } from 'lucide-svelte';
  import { app } from './store.svelte';
  import { searchScripts, kindName } from '$lib/utils/scripts';
  import { api } from './api';
  import type { MaterialSearchResult } from '$lib/types';
  let scripts = $derived(searchScripts(app.nodes, app.query));
  let materials = $state<MaterialSearchResult[]>([]);
  let loading = $state(false);
  let error = $state('');
  $effect(() => {
    const query = app.query;
    let active = true;
    loading = true;
    error = '';
    const timer = setTimeout(() => {
      api<MaterialSearchResult[]>(
        `/search/materials?q=${encodeURIComponent(query)}`,
      )
        .then((result) => {
          if (active) materials = result;
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

<section class="search-page">
  <div class="page-heading">
    <div>
      <span class="eyebrow">搜索结果</span>
      <h2>找到合适的回答</h2>
      <p class="muted">{scripts.length} 条话术 · {materials.length} 份素材</p>
    </div>
    <button class="button secondary" onclick={() => (app.query = '')}
      >返回工作台</button
    >
  </div>
  {#if scripts.length}<div class="section-label">话术</div>
    <div class="search-list">
      {#each scripts.slice(0, 50) as record}<button
          class="search-result"
          onclick={() => app.select(record.node.id)}
          ><span class="result-icon"><MessageSquare size={20} /></span><span
            ><strong>{record.node.title}</strong><small
              >{record.node.replies[app.language] ||
                record.node.question ||
                '此语言暂无回复'}</small
            ><span class="tag"
              >{record.node.category || '未分类'}{record.source === 'personal'
                ? ' · 个人版本'
                : ''}</span
            ></span
          ><ArrowUpRight size={18} /></button
        >{/each}
    </div>{/if}
  {#if materials.length}<div class="section-label">知识库与表格内容</div>
    <div class="search-list">
      {#each materials.slice(0, 50) as { material, rows }}<button
          class="search-result"
          onclick={() => (app.selectedMaterialId = material.id)}
          ><span class="result-icon"
            >{#if material.kind === 'excel'}<FileSpreadsheet
                size={20}
              />{:else}<FileText size={20} />{/if}</span
          ><span
            ><strong>{material.name}</strong><small
              >{material.notes || material.text || material.category}</small
            >{#each rows.slice(0, 3) as row}<span class="excel-hit"
                >{row.sheet} · 第 {row.row} 行：{row.cells.join(' ｜ ')}</span
              >{/each}<span class="tag">{kindName(material.kind)}</span></span
          ><ArrowUpRight size={18} /></button
        >{/each}
    </div>{/if}
  {#if loading}<p class="field-help" role="status">
      正在搜索本地知识库…
    </p>{:else if error}<p class="info-note" role="alert">
      {error}
    </p>{:else if !scripts.length && !materials.length}<div class="empty">
      <MessageSquare size={37} />
      <h3>没有找到匹配内容</h3>
      <p>试试问题中的关键词、产品名、型号或表格单元格内容。</p>
    </div>{/if}
</section>
