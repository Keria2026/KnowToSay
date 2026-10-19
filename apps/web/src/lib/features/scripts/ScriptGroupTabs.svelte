<script lang="ts">
  import { tick } from 'svelte';
  import {
    Plus,
    Pencil,
    X,
    ListOrdered,
    ArrowUp,
    ArrowDown,
    Trash2,
  } from 'lucide-svelte';
  import { orderGroups } from './order';
  import { app } from '../workspace/store.svelte';
  type GroupTab = { id: string; name: string };
  let {
    items,
    selectedId,
    onselect,
    onorder,
    onsave,
    onremove,
    noun = '流程',
    label = '流程切换',
    editable = false,
    renameable = false,
    description = '',
  }: {
    items: GroupTab[];
    selectedId: string;
    onselect: (id: string) => void;
    onorder: (ids: string[]) => boolean;
    onsave?: (id: string, name: string) => Promise<string | undefined>;
    onremove?: (id: string) => Promise<void>;
    noun?: string;
    label?: string;
    editable?: boolean;
    renameable?: boolean;
    description?: string;
  } = $props();
  let tabList: HTMLDivElement;
  let dialog: HTMLDialogElement;
  let name = $state('');
  let editingId = $state('');
  let orderDialog: HTMLDialogElement;
  let orderDraft = $state<string[]>([]);
  let orderPreview = $derived(orderGroups(items, orderDraft));
  let draggingId = $state('');
  let dropId = $state('');
  $effect(() => {
    const selected = selectedId;
    items;
    let cancelled = false;
    void tick().then(() => {
      if (cancelled || selected !== selectedId) return;
      const active = tabList?.querySelector<HTMLButtonElement>(
        '[aria-selected="true"]',
      );
      if (!active) return;
      const bounds = tabList.getBoundingClientRect();
      const button = active.getBoundingClientRect();
      if (button.left < bounds.left)
        tabList.scrollLeft -= bounds.left - button.left;
      else if (button.right > bounds.right)
        tabList.scrollLeft += button.right - bounds.right;
    });
    return () => {
      cancelled = true;
    };
  });
  async function open(flow?: GroupTab) {
    editingId = flow?.id || '';
    name = flow?.name || '';
    await tick();
    dialog.showModal();
  }
  async function save() {
    const id = await onsave?.(editingId, name);
    if (id) {
      dialog.close();
      if (!editingId) onselect(id);
    }
  }

  function moveTab(id: string, target: number) {
    const ids = items.map((f) => f.id);
    const from = ids.indexOf(id);
    if (from < 0 || target < 0 || target >= ids.length || from === target)
      return;
    ids.splice(from, 1);
    ids.splice(target, 0, id);
    if (onorder(ids)) app.notify(`${noun}顺序已保存`);
  }
  function drop(event: DragEvent, target: number) {
    if (!draggingId) return;
    event.preventDefault();
    moveTab(draggingId, target);
    draggingId = '';
    dropId = '';
  }
  async function openOrder() {
    orderDraft = items.map((f) => f.id);
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
    if (onorder(orderPreview.map((f) => f.id))) {
      orderDialog.close();
      app.notify(`${noun}顺序已保存`);
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
      moveTab(items[index].id, target);
      return;
    }
    buttons[next].focus();
    buttons[next].click();
  }
</script>

<div class="named-flow-bar">
  <div
    bind:this={tabList}
    class="named-flow-tabs"
    role="tablist"
    tabindex="-1"
    aria-label={label}
    onkeydown={keyboard}
  >
    {#each items as flow, index (flow.id)}
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
    {#if items.length > 1}<button
        class="icon-button"
        aria-label={`${noun}排序`}
        disabled={!!app.busy}
        onclick={openOrder}><ListOrdered size={17} /></button
      >{/if}
    {#if editable && onsave}
      {#if renameable && items.find((f) => f.id === selectedId)}<button
          class="icon-button"
          disabled={!!app.busy}
          aria-label={`重命名当前${noun}`}
          onclick={() => open(items.find((f) => f.id === selectedId))}
          ><Pencil size={16} /></button
        >{/if}
      {#if renameable && onremove}<button
          class="icon-button"
          aria-label={`删除当前${noun}`}
          disabled={!!app.busy}
          onclick={() => onremove?.(selectedId)}><Trash2 size={16} /></button
        >{/if}
      <button class="text-button" disabled={!!app.busy} onclick={() => open()}
        ><Plus size={17} />新建{noun}</button
      >
    {/if}
  </div>
</div>

<dialog
  bind:this={dialog}
  class="dialog flow-name-dialog"
  aria-labelledby={`${noun}-name-title`}
>
  <form
    onsubmit={(event) => {
      event.preventDefault();
      void save();
    }}
  >
    <div class="dialog-header">
      <h2 id={`${noun}-name-title`}>
        {editingId ? `重命名${noun}` : `新建${noun}`}
      </h2>
      <button
        type="button"
        class="icon-button"
        aria-label={`关闭${noun}编辑`}
        onclick={() => dialog.close()}><X size={18} /></button
      >
    </div>
    <label
      >{noun}名称<input
        bind:value={name}
        required
        maxlength="80"
        placeholder={noun === '流程'
          ? '例如：首次咨询、复购沟通'
          : '例如：产品咨询、价格异议'}
      /></label
    >
    <p class="field-help">{description}</p>
    <div class="button-row">
      <button
        type="button"
        class="button secondary"
        onclick={() => dialog.close()}>取消</button
      ><button class="button primary" disabled={!!app.busy || !name.trim()}
        >保存{noun}名称</button
      >
    </div>
  </form>
</dialog>

<dialog
  bind:this={orderDialog}
  class="dialog flow-name-dialog"
  aria-labelledby={`${noun}-order-title`}
>
  <form
    onsubmit={(event) => {
      event.preventDefault();
      saveOrder();
    }}
  >
    <div class="dialog-header">
      <h2 id={`${noun}-order-title`}>{noun}排序</h2>
      <button
        type="button"
        class="icon-button"
        aria-label={`关闭${noun}排序`}
        onclick={() => orderDialog.close()}><X size={18} /></button
      >
    </div>
    <p class="field-help">
      使用箭头调整先后，也可以直接拖动{noun}标签。顺序仅保存在本机，工作台和编辑器共用。
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
