<script lang="ts">
  import { onMount } from 'svelte';
  import { X, Copy, ArrowUpRight } from 'lucide-svelte';
  import { app } from '../workspace/store.svelte';
  import {
    replyPreview,
    replyVariables,
    languageName,
  } from '$lib/utils/scripts';
  import BilingualReplies from './BilingualReplies.svelte';
  let dialog: HTMLDialogElement;
  let values = $state<Record<string, string>>({});
  let record = $derived(app.nodes.find((r) => r.node.id === app.replyId));
  let preview = $derived(
    record ? replyPreview(record.node, app.language, values) : null,
  );
  let variables = $derived(
    record ? replyVariables(record.node, app.language) : [],
  );
  onMount(() => {
    dialog.showModal();
  });
</script>

<dialog
  bind:this={dialog}
  class="dialog reply-dialog"
  aria-label="话术详情"
  onclose={() => (app.replyId = '')}
>
  <header class="dialog-header">
    <h2>{record?.node.title || '话术已移除'}</h2>
    <button
      class="icon-button"
      aria-label="关闭话术详情"
      onclick={() => dialog.close()}><X size={20} /></button
    >
  </header>
  {#if record}<div class="reply-dialog-body">
      <p class="muted">{record.node.question}</p>
      <BilingualReplies node={record.node} />{#each variables as variable}<label
          >{variable.label || variable.name}{variable.required
            ? ' *'
            : ''}<input
            bind:value={values[variable.name]}
            placeholder={`请输入${variable.label || variable.name}`}
          /></label
        >{/each}
      <p class="field-help">复制预览 · {languageName(app.language)}</p>
      <p class="text-preview">
        {preview?.languageMissing ? '暂无发送语言版本' : preview?.text}
      </p>
      <p class="field-help">
        {preview?.missing.length
          ? `待填写：${preview.missing.join('、')}`
          : '确认后复制发送'}
      </p>
      <div class="button-row">
        <button
          class="button primary"
          disabled={!preview?.ready || !!app.busy}
          onclick={() => preview?.ready && app.copy(preview.text)}
          ><Copy size={16} />复制回复</button
        >{#if !app.compact}<button
            class="text-button"
            onclick={() => {
              if (app.navigate('workbench')) {
                app.select(record!.node.id, true);
                dialog.close();
              }
            }}>在工作台打开<ArrowUpRight size={16} /></button
          >{/if}
      </div>
    </div>{/if}
</dialog>
