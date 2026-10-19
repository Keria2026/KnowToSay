<script lang="ts">
  import { Copy, MessageSquare, ArrowUpRight } from 'lucide-svelte';
  import { app } from '../workspace/store.svelte';
  import { replyPreview, scriptKindName } from '$lib/utils/scripts';
  import type { NodeRecord } from '$lib/types';
  import BilingualReplies from './BilingualReplies.svelte';
  let {
    record,
    active = false,
    oncopy = (text: string) => app.copy(text),
    ondetail = () => {
      app.replyId = record.node.id;
    },
  }: {
    record: NodeRecord;
    active?: boolean;
    oncopy?: (text: string) => Promise<void>;
    ondetail?: () => void;
  } = $props();
  let preview = $derived(replyPreview(record.node, app.language, {}));
</script>

<article class="search-result result-card" class:active>
  <button
    class="result-summary"
    aria-label={`查看${record.node.title}详情`}
    onclick={ondetail}
  >
    <MessageSquare size={18} /><span
      ><strong>{record.node.title}</strong><BilingualReplies
        node={record.node}
      /><span class="tag"
        >{scriptKindName(record.node.kind)} · {record.node.kind === 'quick'
          ? record.node.category || '未分类'
          : app.flows.find((f) => f.id === record.node.flowId)?.name ||
            '流程'}{record.source === 'personal' ? ' · 个人版' : ''}</span
      ></span
    >
  </button>
  <div class="result-actions">
    <button
      class="button primary"
      disabled={!!app.busy || preview.languageMissing}
      aria-label={`${preview.ready ? '复制' : preview.languageMissing ? '缺少发送语言' : '填写变量'}${record.node.title}`}
      onclick={() => (preview.ready ? oncopy(preview.text) : ondetail())}
      ><Copy size={16} />{preview.ready
        ? '复制'
        : preview.languageMissing
          ? '暂无发送语言'
          : '填写变量'}</button
    ><button class="text-button" onclick={ondetail}
      >详情<ArrowUpRight size={15} /></button
    >
  </div>
</article>
