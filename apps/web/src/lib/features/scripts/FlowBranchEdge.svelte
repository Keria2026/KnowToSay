<script lang="ts">
  import {
    BaseEdge,
    getBezierPath,
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
    data,
  }: EdgeProps = $props();
  let path = $derived(
    sourceX < targetX
      ? getBezierPath({
          sourceX,
          sourceY,
          targetX,
          targetY,
          sourcePosition,
          targetPosition,
          curvature: 0.35,
        })
      : getSmoothStepPath({
          sourceX,
          sourceY,
          targetX,
          targetY,
          sourcePosition,
          targetPosition,
          centerY:
            Math.max(sourceY, targetY, Number(data?.routeBottom || 0)) +
            100 +
            Number(data?.branchIndex || 0) * 24,
        }),
  );
</script>

<BaseEdge {id} path={path[0]} {markerEnd} interactionWidth={24} />
