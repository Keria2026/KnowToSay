<script lang="ts">
  import { tick } from 'svelte';
  import {
    Plus,
    Pencil,
    X,
    ListOrdered,
    ArrowUp,
    ArrowDown,
  } from 'lucide-svelte';
  import { orderFlows } from './order';
  import { app } from '../workspace/store.svelte';
  import { api } from '../workspace/api';
  import type { Flow, Snapshot } from '$lib/types';
  let {
    selectedId,
    onselect,
    editable = false,
  }: {
    selectedId: string;
    onselect: (id: string) => void;
    editable?: boolean;
  } = $props();
  let dialog: HTMLDialogElement;
  let name = $state('');
  let editingId = $state('');
  let orderDialog: HTMLDialogElement;
  let orderDraft = $state<string[]>([]);
  let orderPreview = $derived(orderFlows(app.flows, orderDraft));
  let draggingId = $state('');
  let dropId = $state('');
  async function open(flow?: Flow) {
    editingId = flow?.id || '';
    name = flow?.name || '';
    await tick();
    dialog.showModal();
  }
  async function save() {
    const id = editingId || crypto.randomUUID();
    const saved = await app.task(
      '保存流程名称',
      async () => {
        app.data = await api<Snapshot>(
          `/flows/${encodeURIComponent(id)}`,
          'PUT',
          { id, name },
        );
        return true;
      },
      editingId ? '已修改流程名称' : '已新建流程，可在当前页面添加话术',
    );
    if (saved) {
      dialog.close();
      if (!editingId) onselect(id);
    }
  }
  function moveTab(id: string, target: number) {
    const ids = app.flows.map((f) => f.id);
    const from = ids.indexOf(id);
    if (from < 0 || target < 0 || target >= ids.length || from === target)
      return;
    ids.splice(from, 1);
    ids.splice(target, 0, id);
    if (app.saveFlowOrder(ids)) app.notify('流程顺序已保存');
  }
  function drop(event: DragEvent, target: number) {
    if (!draggingId) return;
    event.preventDefault();
    moveTab(draggingId, target);
    draggingId = '';
    dropId = '';
  }
  async function openOrder() {
    orderDraft = app.flows.map((f) => f.id);
    await tick();
    orderDialog.showModal();
  }
  function shiftOrder(id: string, offset: number) {
    const ids = orderPreview.map((f) => f.id);
    const index = ids.indexOf(id);
    const target = index + offset;
    if (index < 0 || target < 0 || target >= ids.length) return;
    ids.splice(index, 1);
    ids.splice(target, 0, id);
    orderDraft = ids;
  }
  function saveOrder() {
    if (app.saveFlowOrder(orderPreview.map((f) => f.id))) {
      orderDialog.close();
      app.notify('流程顺序已保存');
    }
  }
  function keyboard(event: KeyboardEvent) {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
    const buttons = [
      ...(
        event.currentTarget as HTMLDivElement
      ).querySelectorAll<HTMLButtonElement>('[role="tab"]'),
    ];
    const index = buttons.indexOf(event.target as HTMLButtonElement);
    if (index < 0 || !buttons.length) return;
    event.preventDefault();
    const next =
      event.key === 'Home'
        ? 0
        : event.key === 'End'
          ? buttons.length - 1
          : (index + (event.key === 'ArrowRight' ? 1 : -1) + buttons.length) %
            buttons.length;
    if (event.altKey && ['ArrowLeft', 'ArrowRight'].includes(event.key)) {
      const target = index + (event.key === 'ArrowRight' ? 1 : -1);
      moveTab(app.flows[index].id, target);
      return;
    }
    buttons[next].focus();
    buttons[next].click();
  }
</script>

<div class="named-flow-bar">
  <div
    class="named-flow-tabs"
    role="tablist"
    tabindex="-1"
    aria-label="流程切换"
    onkeydown={keyboard}
  >
    {#each app.flows as flow, index (flow.id)}
      <button
        role="tab"
        draggable={!app.busy}
        ondragstart={(event) => {
          draggingId = flow.id;
          if (event.dataTransfer) {
            event.dataTransfer.effectAllowed = 'move';
            event.dataTransfer.setData('text/plain', flow.id);
          }
        }}
        ondragover={(event) => {
          if (draggingId) {
            event.preventDefault();
            dropId = flow.id;
          }
        }}
        ondragleave={() => {
          if (dropId === flow.id) dropId = '';
        }}
        ondrop={(event) => drop(event, index)}
        ondragend={() => {
          draggingId = '';
          dropId = '';
        }}
        class:drop-target={dropId === flow.id && draggingId !== flow.id}
        aria-selected={selectedId === flow.id}
        tabindex={selectedId === flow.id ? 0 : -1}
        class:active={selectedId === flow.id}
        disabled={!!app.busy}
        onclick={() => onselect(flow.id)}>{flow.name}</button
      >
    {/each}
  </div>
  <div class="button-row">
    {#if app.flows.length > 1}<button
        class="icon-button"
        aria-label="流程排序"
        disabled={!!app.busy}
        onclick={openOrder}><ListOrdered size={17} /></button
      >{/if}
    {#if editable}
      {#if app.flows.find((f) => f.id === selectedId)}<button
          class="icon-button"
          disabled={!!app.busy}
          aria-label="重命名当前流程"
          onclick={() => open(app.flows.find((f) => f.id === selectedId))}
          ><Pencil size={16} /></button
        >{/if}
      <button class="text-button" disabled={!!app.busy} onclick={() => open()}
        ><Plus size={17} />新建流程</button
      >
    {/if}
  </div>
</div>

<dialog
  bind:this={dialog}
  class="dialog flow-name-dialog"
  aria-labelledby="flow-name-title"
>
  <form
    onsubmit={(event) => {
      event.preventDefault();
      void save();
    }}
  >
    <div class="dialog-header">
      <h2 id="flow-name-title">{editingId ? '重命名流程' : '新建流程'}</h2>
      <button
        type="button"
        class="icon-button"
        aria-label="关闭流程编辑"
        onclick={() => dialog.close()}><X size={18} /></button
      >
    </div>
    <label
      >流程名称<input
        bind:value={name}
        required
        maxlength="80"
        placeholder="例如：首次咨询、复购沟通"
      /></label
    >
    <p class="field-help">
      每条流程拥有独立页面，在画布上方切换。{app.admin
        ? '发布后统一同步给业务员。'
        : '个人流程仅保存在本机。'}
    </p>
    <div class="button-row">
      <button
        type="button"
        class="button secondary"
        onclick={() => dialog.close()}>取消</button
      ><button class="button primary" disabled={!!app.busy || !name.trim()}
        >保存流程名称</button
      >
    </div>
  </form>
</dialog>

<dialog
  bind:this={orderDialog}
  class="dialog flow-name-dialog"
  aria-labelledby="flow-order-title"
>
  <form
    onsubmit={(event) => {
      event.preventDefault();
      saveOrder();
    }}
  >
    <div class="dialog-header">
      <h2 id="flow-order-title">流程排序</h2>
      <button
        type="button"
        class="icon-button"
        aria-label="关闭流程排序"
        onclick={() => orderDialog.close()}><X size={18} /></button
      >
    </div>
    <p class="field-help">
      使用箭头调整先后，也可以直接拖动流程标签。顺序仅保存在本机，工作台和编辑器共用。
    </p>
    <ol class="flow-order-list">
      {#each orderPreview as flow, i (flow.id)}<li>
          <span>{flow.name}</span>
          <button
            type="button"
            class="icon-button"
            aria-label={`向前移动${flow.name}`}
            disabled={i === 0 || !!app.busy}
            onclick={() => shiftOrder(flow.id, -1)}
            ><ArrowUp size={17} /></button
          >
          <button
            type="button"
            class="icon-button"
            aria-label={`向后移动${flow.name}`}
            disabled={i === orderPreview.length - 1 || !!app.busy}
            onclick={() => shiftOrder(flow.id, 1)}
            ><ArrowDown size={17} /></button
          >
        </li>{/each}
    </ol>
    <div class="button-row">
      <button
        type="button"
        class="button secondary"
        onclick={() => orderDialog.close()}>取消</button
      ><button class="button primary" disabled={!!app.busy}>保存排序</button>
    </div>
  </form>
</dialog>
