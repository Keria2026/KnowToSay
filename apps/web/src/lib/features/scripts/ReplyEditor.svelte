<script lang="ts">
  import { Plus, X, ArrowUp, ArrowDown } from 'lucide-svelte';
  import type { ScriptNode } from '$lib/types';
  import { languageName, repliesFor } from '$lib/utils/scripts';
  import ReplySelector from './ReplySelector.svelte';
  let { node = $bindable() }: { node: ScriptNode } = $props();
  let selected = $state('');
  let replyId = $derived(
    node.replyOptions?.some((r) => r.id === selected) ? selected : '',
  );
  let replies = $derived(repliesFor(node, replyId));
  let option = $derived(node.replyOptions?.find((r) => r.id === replyId));
  function add() {
    const id = crypto.randomUUID();
    node.replyOptions = [
      ...(node.replyOptions ?? []),
      {
        id,
        name: `回复 ${(node.replyOptions?.length ?? 0) + 2}`,
        replies: { 'zh-CN': '', ko: '' },
      },
    ];
    selected = id;
  }
  function move(delta: number) {
    const options = [...(node.replyOptions ?? [])];
    const i = options.findIndex((r) => r.id === replyId);
    const next = i + delta;
    if (i < 0 || next < 0 || next >= options.length) return;
    [options[i], options[next]] = [options[next], options[i]];
    node.replyOptions = options;
  }
</script>

<div class="editor-section">
  <div class="section-title">
    <h3>话术内容</h3>
    {#if node.kind === 'quick'}<button
        type="button"
        class="text-button"
        disabled={(node.replyOptions?.length ?? 0) >= 49}
        onclick={add}><Plus size={14} />添加回复</button
      >{/if}
  </div>
  <ReplySelector {node} bind:replyId={selected} />
  {#if option}
    <label
      >回复名称<input bind:value={option.name} required maxlength="80" /></label
    >
    <div class="button-row">
      <button
        type="button"
        class="icon-button"
        aria-label="上移回复"
        disabled={node.replyOptions?.[0]?.id === replyId}
        onclick={() => move(-1)}><ArrowUp size={16} /></button
      >
      <button
        type="button"
        class="icon-button"
        aria-label="下移回复"
        disabled={node.replyOptions?.at(-1)?.id === replyId}
        onclick={() => move(1)}><ArrowDown size={16} /></button
      >
      <button
        type="button"
        class="text-button"
        onclick={() => {
          node.replyOptions =
            node.replyOptions?.filter((r) => r.id !== replyId) ?? [];
          selected = '';
        }}><X size={14} />删除此回复</button
      >
    </div>
  {/if}
  {#each ['zh-CN', 'ko', ...Object.keys(replies).filter((key) => !['zh-CN', 'ko'].includes(key))] as language}
    <label
      >{languageName(language)}<textarea
        bind:value={replies[language]}
        rows="4"
        placeholder={'输入完整回复，可使用 {变量名}'}
      ></textarea></label
    >
  {/each}
</div>
