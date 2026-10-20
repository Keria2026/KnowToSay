<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { X, Upload, ArrowUp, ArrowDown } from 'lucide-svelte';
  import { app } from '../workspace/store.svelte';
  import {
    api,
    native,
    selectNativeFiles,
    uploadFiles,
  } from '../workspace/api';
  import { folderPath } from './folders';
  import MaterialImage from './MaterialImage.svelte';
  import type { Material, Snapshot } from '$lib/types';
  let {
    material,
    folderId = null,
    onclose,
  }: {
    material?: Material;
    folderId?: string | null;
    onclose: () => void;
  } = $props();
  let dialog: HTMLDialogElement;
  let fileInput: HTMLInputElement;
  let draft = $state<Material>(
    untrack(() =>
      material
        ? structuredClone($state.snapshot(material))
        : {
            id: crypto.randomUUID(),
            name: '',
            kind: 'case',
            category: '',
            folderId,
            imageIds: [],
            text: '',
            notes: '',
            tags: [],
            hash: null,
            size: 0,
            mime: 'text/plain',
            rows: [],
          },
    ),
  );
  let tags = $state(draft.tags.join('，'));
  onMount(() => dialog.showModal());
  async function upload(files?: FileList | null) {
    await app.task('导入案例图片', async () => {
      const imported = native()
        ? await selectNativeFiles(true)
        : files?.length
          ? await uploadFiles(files)
          : [];
      draft.imageIds = [
        ...(draft.imageIds ?? []),
        ...imported.filter((m) => m.kind === 'image').map((m) => m.id),
      ];
      await app.refresh();
    });
    if (fileInput) fileInput.value = '';
  }
  function move(index: number, delta: number) {
    const ids = [...(draft.imageIds ?? [])];
    [ids[index], ids[index + delta]] = [ids[index + delta], ids[index]];
    draft.imageIds = ids;
  }
  async function save() {
    const saved = await app.task('保存案例', async () => {
      app.data = await api<Snapshot>('/materials', 'POST', {
        ...draft,
        tags: tags
          .split(/[,，]/)
          .map((s) => s.trim())
          .filter(Boolean),
      });
      return true;
    });
    if (saved) dialog.close();
  }
</script>

<dialog
  bind:this={dialog}
  class="case-editor edit-material-dialog"
  {onclose}
  aria-label="编辑案例"
>
  <header class="dialog-header">
    <h2>{material ? '编辑案例' : '上传案例'}</h2>
    <button
      type="button"
      class="icon-button"
      aria-label="关闭案例编辑"
      onclick={() => dialog.close()}><X size={20} /></button
    >
  </header>
  <form
    onsubmit={(event) => {
      event.preventDefault();
      void save();
    }}
  >
    <label
      >案例名称<input bind:value={draft.name} required maxlength="160" /></label
    >
    <label
      >所在文件夹<select aria-label="所在文件夹" bind:value={draft.folderId}
        ><option value={null}>资料根目录</option
        >{#each app.folders as folder}<option value={folder.id}
            >{folderPath(app.folders, folder.id)}</option
          >{/each}</select
      ></label
    >
    <label
      >案例文案<textarea bind:value={draft.text} rows="6" required
      ></textarea></label
    >
    <label
      >标签<input bind:value={tags} placeholder="多个标签用逗号分隔" /></label
    >
    <label>备注<textarea bind:value={draft.notes} rows="2"></textarea></label>
    <div class="section-title">
      <h3>组合图片 · {draft.imageIds?.length ?? 0}</h3>
      <button
        type="button"
        class="text-button"
        disabled={!!app.busy || (draft.imageIds?.length ?? 0) >= 50}
        onclick={() => (native() ? upload() : fileInput.click())}
        ><Upload size={16} />添加案例图片</button
      >
    </div>
    <input
      class="visually-hidden"
      type="file"
      accept="image/*"
      multiple
      bind:this={fileInput}
      onchange={(event) => upload(event.currentTarget.files)}
    />
    <div class="case-image-grid">
      {#each draft.imageIds ?? [] as id, i}{@const image = app.materials.find(
          (m) => m.id === id,
        )}
        <div class="case-image-tile">
          {#if image}<MaterialImage material={image} />{:else}<p>
              图片已移除
            </p>{/if}
          <div class="inline-actions">
            <button
              type="button"
              class="icon-button"
              aria-label={`上移案例图片 ${i + 1}`}
              disabled={i === 0}
              onclick={() => move(i, -1)}><ArrowUp size={15} /></button
            ><button
              type="button"
              class="icon-button"
              aria-label={`下移案例图片 ${i + 1}`}
              disabled={i === (draft.imageIds?.length ?? 0) - 1}
              onclick={() => move(i, 1)}><ArrowDown size={15} /></button
            ><button
              type="button"
              class="icon-button"
              aria-label={`移除案例图片 ${i + 1}`}
              onclick={() =>
                (draft.imageIds = draft.imageIds?.filter((x) => x !== id))}
              ><X size={15} /></button
            >
          </div>
        </div>
      {/each}
    </div>
    <div class="button-row">
      <button
        type="button"
        class="button secondary"
        onclick={() => dialog.close()}>取消</button
      ><button
        class="button primary"
        disabled={!!app.busy ||
          !draft.name.trim() ||
          !draft.text.trim() ||
          !draft.imageIds?.length}>保存案例</button
      >
    </div>
  </form>
</dialog>
