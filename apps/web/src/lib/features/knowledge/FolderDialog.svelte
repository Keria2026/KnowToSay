<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { FolderPlus, X } from 'lucide-svelte';
  import { app } from '../workspace/store.svelte';
  import { api } from '../workspace/api';
  import { folderPath, folderDescendants } from './folders';
  import type { Folder, Snapshot } from '$lib/types';
  let {
    folder,
    parentId = null,
    onclose,
    onsave,
  }: {
    folder?: Folder;
    parentId?: string | null;
    onclose: () => void;
    onsave: (id: string) => void;
  } = $props();
  let dialog: HTMLDialogElement;
  let name = $state(untrack(() => folder?.name ?? ''));
  let parent = $state(
    untrack(() => (folder ? (folder.parentId ?? null) : parentId)),
  );
  let error = $state('');
  let choices = $derived(
    app.folders.filter(
      (f) => !folder || !folderDescendants(app.folders, folder.id).has(f.id),
    ),
  );
  onMount(() => dialog.showModal());
  async function save() {
    error = '';
    const id = folder?.id ?? crypto.randomUUID();
    const saved = await app.task('保存文件夹', async () => {
      app.data = await api<Snapshot>('/folders', 'POST', {
        id,
        name: name.trim(),
        parentId: parent,
      });
      return true;
    });
    if (saved) {
      onsave(id);
      dialog.close();
    } else if (app.error) {
      error = app.error;
      app.error = '';
    }
  }
</script>

<dialog
  bind:this={dialog}
  class="edit-material-dialog"
  {onclose}
  aria-label={folder ? '管理文件夹' : '新建文件夹'}
>
  <header class="dialog-header">
    <h2>{folder ? '管理文件夹' : '新建文件夹'}</h2>
    <button
      type="button"
      class="icon-button"
      aria-label="关闭文件夹设置"
      onclick={() => dialog.close()}><X size={20} /></button
    >
  </header>
  <form
    onsubmit={(event) => {
      event.preventDefault();
      void save();
    }}
  >
    <label>文件夹名称<input bind:value={name} required maxlength="80" /></label>
    <label
      >上级文件夹<select aria-label="上级文件夹" bind:value={parent}
        ><option value={null}>资料根目录</option>{#each choices as f}<option
            value={f.id}>{folderPath(app.folders, f.id)}</option
          >{/each}</select
      ></label
    >
    {#if error}<p role="alert" class="info-note">{error}</p>{/if}
    <div class="button-row">
      <button
        type="button"
        class="button secondary"
        onclick={() => dialog.close()}>取消</button
      ><button class="button primary" disabled={!name.trim() || !!app.busy}
        ><FolderPlus size={16} />{folder ? '保存文件夹' : '创建文件夹'}</button
      >
    </div>
  </form>
</dialog>
