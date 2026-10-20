<script lang="ts">
  import type { ScriptNode } from '$lib/types';
  import { replyChoices } from '$lib/utils/scripts';
  let {
    node,
    replyId = $bindable(''),
  }: { node: ScriptNode; replyId?: string } = $props();
</script>

{#if node.replyOptions?.length}
  <label class="reply-selector"
    >选择回复
    <select bind:value={replyId} aria-label="选择回复">
      {#each replyChoices(node) as reply (reply.id)}<option value={reply.id}
          >{reply.name}</option
        >{/each}
    </select>
  </label>
{/if}
