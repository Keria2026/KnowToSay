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
  async function save(_id: string, name: string) {
    return app.task(
      '新建快捷话术分类',
      async () => {
        app.data = await api<Snapshot>('/quick-categories', 'POST', { name });
        return name.trim();
      },
      app.admin ? '已新建分类，发布后同步给业务员' : '已新建个人分类',
    );
  }
</script>

<ScriptGroupTabs
  items={app.quickGroups}
  {selectedId}
  {onselect}
  {editable}
  noun="分类"
  label="快捷话术分类"
  onorder={(ids) => app.saveQuickCategoryOrder(ids)}
  onsave={save}
  description={app.admin
    ? '分类独立保存，发布后同步给业务员。'
    : '个人分类保存在本机，不影响管理员主库。'}
/>
