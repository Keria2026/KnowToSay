<script lang="ts">
  import { GitBranch, MessageSquare } from 'lucide-svelte';
  import { app } from '../workspace/store.svelte';
  import type { ScriptKind } from '$lib/types';
  import FlowTabs from './FlowTabs.svelte';
  import QuickCategoryTabs from './QuickCategoryTabs.svelte';
  let {
    kind,
    onkind,
    onflow,
    oncategory,
    oncategorychange,
    editable = false,
    flowCount = app.nodes.filter((r) => r.node.kind !== 'quick').length,
    quickCount = app.nodes.filter((r) => r.node.kind === 'quick').length,
  }: {
    kind: ScriptKind;
    onkind: (kind: ScriptKind) => void;
    onflow: (id: string) => void;
    oncategory: (name: string) => void;
    oncategorychange?: (oldName: string, name: string) => void;
    editable?: boolean;
    flowCount?: number;
    quickCount?: number;
  } = $props();
</script>

<div class="script-navigation" aria-label="话术导航">
  <div class="script-type-tabs" role="tablist" aria-label="话术类型">
    <button
      role="tab"
      aria-selected={kind === 'flow'}
      class:active={kind === 'flow'}
      disabled={!!app.busy}
      onclick={() => onkind('flow')}
      ><GitBranch size={17} />流程话术 <span>{flowCount}</span></button
    >
    <button
      role="tab"
      aria-selected={kind === 'quick'}
      class:active={kind === 'quick'}
      disabled={!!app.busy}
      onclick={() => onkind('quick')}
      ><MessageSquare size={17} />快捷话术 <span>{quickCount}</span></button
    >
  </div>
  {#if kind === 'flow'}<FlowTabs
      selectedId={app.activeFlowId}
      onselect={onflow}
      {editable}
    />
  {:else}<QuickCategoryTabs
      selectedId={app.activeQuickCategory}
      onselect={oncategory}
      onchange={oncategorychange}
      {editable}
    />{/if}
</div>
