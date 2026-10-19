<script lang="ts">
  import type { ScriptNode } from '$lib/types';
  let { node }: { node: ScriptNode } = $props();
</script>

<span class="bilingual-replies">
  <span class="bilingual-reply" lang="zh-CN"
    ><span class="reply-language">中文</span><span
      >{node.replies['zh-CN']?.trim() || '暂无中文回复'}</span
    ></span
  >
  <span class="bilingual-reply" lang="ko"
    ><span class="reply-language">한국어</span><span
      >{node.replies.ko?.trim() || '暂无韩语回复'}</span
    ></span
  >
</span>
