<script lang="ts">
  import type { ScriptNode } from '$lib/types';
  import { repliesFor } from '$lib/utils/scripts';
  let { node, replyId = '' }: { node: ScriptNode; replyId?: string } = $props();
  let replies = $derived(repliesFor(node, replyId));
</script>

<span class="bilingual-replies">
  <span class="bilingual-reply" lang="zh-CN"
    ><span class="reply-language">中文</span><span
      >{replies['zh-CN']?.trim() || '暂无中文回复'}</span
    ></span
  >
  <span class="bilingual-reply" lang="ko"
    ><span class="reply-language">한국어</span><span
      >{replies.ko?.trim() || '暂无韩语回复'}</span
    ></span
  >
</span>
