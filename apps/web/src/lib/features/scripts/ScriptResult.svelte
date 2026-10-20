<script lang="ts">
  import { Copy, MessageSquare, ArrowUpRight } from 'lucide-svelte';
  import { app } from '../workspace/store.svelte';
  import {
    replyPreview,
    scriptKindName,
    matchingReply,
  } from '$lib/utils/scripts';
  import type { NodeRecord } from '$lib/types';
  import ReplySelector from './ReplySelector.svelte';
  import BilingualReplies from './BilingualReplies.svelte';
  let {
    record,
    active = false,
    query = '',
    oncopy = (text: string) => app.copy(text),
    ondetail = (replyId: string) => {
      app.replyOptionId = replyId;
      app.replyId = record.node.id;
    },
  }: {
    record: NodeRecord;
    active?: boolean;
    query?: string;
    oncopy?: (text: string) => Promise<void>;
    ondetail?: (replyId: string) => void;
  } = $props();
  let replyId = $state('');
  $effect(() => {
    replyId = matchingReply(record.node, query);
  });
  let preview = $derived(replyPreview(record.node, app.language, {}, replyId));
</script>

<article class="search-result result-card" class:active>
  <button
    class="result-summary"
    aria-label={`查看${record.node.title}详情`}
    onclick={() => ondetail(replyId)}
  >
    <MessageSquare size={18} /><span
      ><strong>{record.node.title}</strong><BilingualReplies
        node={record.node}
        {replyId}
      /><span class="tag"
        >{scriptKindName(record.node.kind)} · {record.node.kind === 'quick'
          ? record.node.category || '未分类'
          : app.flows.find((f) => f.id === record.node.flowId)?.name ||
            '流程'}{record.source === 'personal' ? ' · 个人版' : ''}</span
      ></span
    >
  </button>
  <div class="result-actions">
    <ReplySelector node={record.node} bind:replyId />
    <button
      class="button primary"
      disabled={!!app.busy || preview.languageMissing}
      aria-label={`${preview.ready ? '复制' : preview.languageMissing ? '缺少发送语言' : '填写变量'}${record.node.title}`}
      onclick={() => (preview.ready ? oncopy(preview.text) : ondetail(replyId))}
      ><Copy size={16} />{preview.ready
        ? '复制'
        : preview.languageMissing
          ? '暂无发送语言'
          : '填写变量'}</button
    ><button class="text-button" onclick={() => ondetail(replyId)}
      >详情<ArrowUpRight size={15} /></button
    >
  </div>
</article>
