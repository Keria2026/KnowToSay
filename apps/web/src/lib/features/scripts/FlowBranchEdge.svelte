<script lang="ts">
  import {
    BaseEdge,
    EdgeLabel,
    getSmoothStepPath,
    type EdgeProps,
  } from '@xyflow/svelte';
  let {
    id,
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
    markerEnd,
    label,
    data,
  }: EdgeProps = $props();
  let path = $derived(
    getSmoothStepPath({
      sourceX,
      sourceY,
      targetX,
      targetY,
      sourcePosition,
      targetPosition,
      // Route backward jumps below both steps so their labels do not cover
      // the forward branch between the same two steps.
      centerY: sourceX >= targetX ? Math.max(sourceY, targetY) + 96 : undefined,
    }),
  );
</script>

<BaseEdge {id} path={path[0]} {markerEnd} interactionWidth={24} />
<EdgeLabel x={path[1]} y={path[2]} transparent>
  {#if typeof data?.onEdit === 'function'}
    <button
      class="flow-branch-label nodrag nopan"
      aria-label={`编辑分支：${label}`}
      onclick={(event) => {
        event.stopPropagation();
        if (typeof data?.onEdit === 'function') data.onEdit();
      }}>{label}</button
    >
  {:else}<span class="flow-branch-text">{label}</span>{/if}
</EdgeLabel>
