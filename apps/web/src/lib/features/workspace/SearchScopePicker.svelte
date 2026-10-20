<script lang="ts">
  import { app } from './store.svelte';
  import { folderPath } from '../knowledge/folders';
  import type { SearchScope, SearchArea } from './search';
  let { scope = $bindable() }: { scope: SearchScope } = $props();
  let groups = $derived(
    scope.area === 'flow'
      ? app.flows
      : scope.area === 'quick'
        ? app.quickGroups
        : app.folders.map((folder) => ({
            id: folder.id,
            name: folderPath(app.folders, folder.id),
          })),
  );
</script>

<div class="search-scope-picker">
  <select
    aria-label="搜索范围"
    value={scope.area}
    onchange={(e) =>
      (scope = { area: e.currentTarget.value as SearchArea, group: '' })}
  >
    <option value="all">全局搜索</option><option value="flow">流程话术</option
    ><option value="quick">快捷话术</option><option value="library"
      >知识库</option
    >
  </select>
  {#if scope.area !== 'all'}<select
      aria-label="搜索分组"
      value={scope.group}
      onchange={(e) => (scope = { ...scope, group: e.currentTarget.value })}
      ><option value=""
        >全部{scope.area === 'flow'
          ? '流程'
          : scope.area === 'library'
            ? '文件夹'
            : '分类'}</option
      >{#each groups as group}<option value={group.id}>{group.name}</option
        >{/each}</select
    >{/if}
</div>
