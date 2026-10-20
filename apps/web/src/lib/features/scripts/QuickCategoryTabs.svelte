<script lang="ts">
  import ScriptGroupTabs from './ScriptGroupTabs.svelte';
  import { app } from '../workspace/store.svelte';
  import { api } from '../workspace/api';
  import type { Snapshot } from '$lib/types';
  let {
    selectedId,
    onselect,
    editable = false,
    onchange,
  }: {
    selectedId: string;
    onselect: (id: string) => void;
    editable?: boolean;
    onchange?: (oldName: string, name: string) => void;
  } = $props();
  function changed(oldName: string, name: string) {
    const order = app.quickCategoryOrder.map((id) =>
      id === oldName ? name : id,
    );
    app.quickCategory = name;
    app.saveQuickCategoryOrder(order);
    onchange?.(oldName, name);
  }
  async function save(id: string, name: string) {
    const next = name.trim();
    return app.task(
      id ? '重命名快捷话术分类' : '新建快捷话术分类',
      async () => {
        app.data = await api<Snapshot>(
          id
            ? `/quick-categories/${encodeURIComponent(id)}`
            : '/quick-categories',
          id ? 'PUT' : 'POST',
          { name },
        );
        if (id) changed(id, next);
        return next;
      },
      app.admin ? '分类已保存，发布后同步给业务员' : '个人分类已保存',
    );
  }
  async function remove(id: string) {
    if (!confirm(`删除分类“${id}”？其中的话术将移到“未分类”，话术内容会保留。`))
      return;
    await app.task(
      '删除快捷话术分类',
      async () => {
        app.data = await api<Snapshot>(
          `/quick-categories/${encodeURIComponent(id)}`,
          'DELETE',
        );
        changed(id, '未分类');
      },
      '分类已删除，话术已移到未分类',
    );
  }
</script>

<ScriptGroupTabs
  items={app.quickGroups}
  {selectedId}
  {onselect}
  {editable}
  renameable={!!app.data?.manageableQuickCategories?.includes(selectedId)}
  onremove={remove}
  noun="分类"
  label="快捷话术分类"
  onorder={(ids) => app.saveQuickCategoryOrder(ids)}
  onsave={save}
  description={app.admin
    ? '分类独立保存，发布后同步给业务员。'
    : '个人分类保存在本机，不影响管理员主库。'}
/>
