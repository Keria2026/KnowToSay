<script lang="ts">
  import { onMount } from 'svelte';
  import { FolderPlus, X } from 'lucide-svelte';
  import { app } from '../workspace/store.svelte';
  import { api } from '../workspace/api';
  import type { Snapshot } from '$lib/types';

  let {
    onclose,
    oncreate,
  }: {
    onclose: () => void;
    oncreate: (name: string) => void;
  } = $props();
  let dialog: HTMLDialogElement;
  let input: HTMLInputElement;
  let name = $state('');
  let error = $state('');
  onMount(() => {
    dialog.showModal();
    input.focus();
  });
  async function create() {
    const trimmed = name.trim();
    if (!trimmed || app.busy) return;
    error = '';
    await app.task(
      '新建分类',
      async () => {
        app.data = await api<Snapshot>('/material-categories', 'POST', {
          name: trimmed,
        });
        oncreate(trimmed);
        dialog.close();
      },
      '分类已创建，发布后同步给团队',
    );
    if (app.error) {
      error = app.error;
      app.error = '';
    }
  }
</script>

<dialog
  bind:this={dialog}
  class="edit-material-dialog"
  {onclose}
  aria-labelledby="category-dialog-title"
>
  <header class="dialog-header">
    <div>
      <span class="eyebrow">KNOWLEDGE LIBRARY</span>
      <h2 id="category-dialog-title">新建分类</h2>
    </div>
    <button
      class="icon-button"
      aria-label="关闭新建分类"
      onclick={() => dialog.close()}><X size={20} /></button
    >
  </header>
  <form
    onsubmit={(event) => {
      event.preventDefault();
      void create();
    }}
  >
    <label
      >分类名称<input
        bind:this={input}
        bind:value={name}
        required
        maxlength="80"
        placeholder="例如：产品资料、配送与售后"
      /></label
    >
    <p class="field-help">
      分类由管理员统一维护，发布后同步给团队。暂时没有素材的分类也会保留。
    </p>
    {#if error}<p class="info-note" role="alert">{error}</p>{/if}
    <div class="button-row">
      <button
        class="button secondary"
        type="button"
        onclick={() => dialog.close()}>取消</button
      >
      <button class="button primary" disabled={!name.trim() || !!app.busy}
        ><FolderPlus size={16} />创建分类</button
      >
    </div>
  </form>
</dialog>
