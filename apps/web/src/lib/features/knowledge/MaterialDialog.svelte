<script lang="ts">
  import { onMount } from 'svelte';
  import { X } from 'lucide-svelte';
  import { app } from '../workspace/store.svelte';
  import MaterialPreview from './MaterialPreview.svelte';
  let dialog: HTMLDialogElement;
  let material = $derived(
    app.materials.find((m) => m.id === app.selectedMaterialId),
  );
  onMount(() => dialog.showModal());
</script>

<dialog
  bind:this={dialog}
  class="material-dialog"
  onclose={() => {
    app.selectedMaterialId = '';
    app.materialQuery = null;
  }}
>
  <header class="dialog-header">
    <div>
      <span class="eyebrow">知识库素材</span>
      <h2>{material?.name || '素材已移除'}</h2>
    </div>
    <button
      class="icon-button"
      aria-label="关闭预览"
      onclick={() => dialog.close()}><X size={21} /></button
    >
  </header>
  {#if material}<MaterialPreview
      {material}
      query={app.materialQuery ?? app.query}
    />{:else}<p>
      管理员已移除此素材。个人话术保持不变，你可以重新选择关联素材。
    </p>{/if}
</dialog>
