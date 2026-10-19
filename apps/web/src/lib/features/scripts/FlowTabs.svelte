<script lang="ts">
  import { tick } from 'svelte';
  import { Plus, Pencil, X } from 'lucide-svelte';
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
    {#each app.flows as flow (flow.id)}
      <button
        role="tab"
        aria-selected={selectedId === flow.id}
        tabindex={selectedId === flow.id ? 0 : -1}
        class:active={selectedId === flow.id}
        disabled={!!app.busy}
        onclick={() => onselect(flow.id)}>{flow.name}</button
      >
    {/each}
  </div>
  {#if editable}<div class="button-row">
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
    </div>{/if}
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
