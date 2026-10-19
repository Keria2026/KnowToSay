<script lang="ts">
  import { app } from '../workspace/store.svelte';
  import { api } from '../workspace/api';
  import type { SheetPage } from '$lib/types';
  import { columnName } from './sheet';
  let { materialId, query = '' }: { materialId: string; query?: string } =
    $props();
  let sheet = $state('');
  let offset = $state(0);
  let columnOffset = $state(0);
  let page = $state<SheetPage | null>(null);
  let loading = $state(false);
  let error = $state('');
  let key = '';
  let columns = $derived(
    Array.from(
      {
        length: Math.min(
          40,
          Math.max(0, (page?.columnCount ?? 0) - columnOffset),
        ),
      },
      (_, i) => i + columnOffset,
    ),
  );
  $effect(() => {
    const nextKey = `${materialId}:${query}:${sheet}`;
    if (key !== nextKey) {
      key = nextKey;
      offset = 0;
      columnOffset = 0;
    }
    const start = offset;
    const q = query;
    const selectedSheet = sheet;
    let active = true;
    loading = true;
    error = '';
    page = null;
    const params = new URLSearchParams({ q, offset: String(start) });
    if (selectedSheet) params.set('sheet', selectedSheet);
    void api<SheetPage>(
      `/materials/${encodeURIComponent(materialId)}/rows?${params}`,
    )
      .then((result) => {
        if (!active) return;
        page = result;
        if (q && start === 0) {
          const index =
            result.rows[0]?.cells.findIndex((c) =>
              c.toLocaleLowerCase().includes(q.toLocaleLowerCase()),
            ) ?? -1;
          if (index >= 0) columnOffset = Math.floor(index / 40) * 40;
        }
      })
      .catch((e) => {
        if (active) error = String(e);
      })
      .finally(() => {
        if (active) loading = false;
      });
    return () => {
      active = false;
    };
  });
  let sheets = $state<string[]>([]);
  $effect(() => {
    if (page) sheets = page.sheets;
  });
</script>

<div class="sheet-toolbar">
  <label
    >工作表<select aria-label="预览工作表" bind:value={sheet}
      ><option value="">所有工作表</option>{#each sheets as name}<option
          value={name}>{name}</option
        >{/each}</select
    ></label
  >
  <p class="field-help">点击单元格复制内容；可横向滚动查看列。</p>
</div>
{#if loading}<p class="field-help" role="status">
    正在读取表格…
  </p>{:else if error}<p class="info-note" role="alert">
    {error}
  </p>{:else if page}
  <div
    class="table-scroll spreadsheet-scroll"
    role="region"
    aria-label="表格内容"
  >
    <table class="spreadsheet">
      <thead
        ><tr
          ><th scope="col">{sheet ? '行号' : '工作表 / 行'}</th
          >{#each columns as col}<th scope="col">{columnName(col)}</th
            >{/each}<th scope="col">操作</th></tr
        ></thead
      ><tbody
        >{#each page.rows as row}<tr
            ><th scope="row"
              >{#if !sheet}{row.sheet} ·
              {/if}{row.row}</th
            >{#each columns as col}{@const cell = row.cells[col] || ''}<td
                class:cell-match={!!query &&
                  cell.toLocaleLowerCase().includes(query.toLocaleLowerCase())}
                ><button
                  class="sheet-cell"
                  disabled={!cell || !!app.busy}
                  aria-label={`复制 ${columnName(col)}${row.row}`}
                  title={cell}
                  onclick={() => app.copy(cell)}>{cell || ' '}</button
                ></td
              >{/each}<td
              ><button
                class="text-button"
                aria-label={`复制${row.sheet}第${row.row}行`}
                disabled={!!app.busy}
                onclick={() => app.copy(row.cells.join('\t'))}>复制行</button
              ></td
            ></tr
          >{/each}</tbody
      >
    </table>
  </div>
  {#if !page.rows.length}<p class="field-help">
      此工作表没有{query ? '匹配内容' : '非空行'}。
    </p>{/if}
  <div class="sheet-pagination">
    <p class="field-help">
      共 {page.total} 个{query ? '匹配' : '非空'}行，当前 {page.rows.length
        ? offset + 1
        : 0}–{offset + page.rows.length} 行
    </p>
    <div class="button-row">
      <button
        class="button secondary"
        disabled={offset === 0}
        onclick={() => (offset = Math.max(0, offset - 150))}>上一页</button
      ><button
        class="button secondary"
        disabled={offset + page.rows.length >= page.total}
        onclick={() => (offset += 150)}>下一页</button
      >
    </div>
  </div>
  {#if page.columnCount > 40}<div class="sheet-pagination">
      <span class="field-help"
        >列 {columnName(columnOffset)}–{columnName(
          columnOffset + columns.length - 1,
        )} / 共 {page.columnCount} 列</span
      >
      <div class="button-row">
        <button
          class="text-button"
          disabled={columnOffset === 0}
          onclick={() => (columnOffset = Math.max(0, columnOffset - 40))}
          >上一组列</button
        ><button
          class="text-button"
          disabled={columnOffset + 40 >= page.columnCount}
          onclick={() => (columnOffset += 40)}>下一组列</button
        >
      </div>
    </div>{/if}
{/if}
