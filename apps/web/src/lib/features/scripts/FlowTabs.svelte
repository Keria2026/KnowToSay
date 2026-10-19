<script lang="ts">
  import ScriptGroupTabs from './ScriptGroupTabs.svelte';
  import { app } from '../workspace/store.svelte';
  import { api } from '../workspace/api';
  import type { Snapshot } from '$lib/types';
  let {
    selectedId,
    onselect,
    editable = false,
  }: {
    selectedId: string;
    onselect: (id: string) => void;
    editable?: boolean;
  } = $props();
  async function save(id: string, name: string) {
    const flowId = id || crypto.randomUUID();
    return app.task(
      '保存流程名称',
      async () => {
        app.data = await api<Snapshot>(
          `/flows/${encodeURIComponent(flowId)}`,
          'PUT',
          { id: flowId, name },
        );
        return flowId;
      },
      id ? '已修改流程名称' : '已新建流程，可在当前页面添加话术',
    );
  }
</script>

<ScriptGroupTabs
  items={app.flows}
  {selectedId}
  {onselect}
  {editable}
  renameable
  onorder={(ids) => app.saveFlowOrder(ids)}
  onsave={save}
  description={`每条流程拥有独立页面，在上方切换。${app.admin ? '发布后统一同步给业务员。' : '个人流程仅保存在本机。'}`}
/>
