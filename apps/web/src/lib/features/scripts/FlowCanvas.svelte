<script lang="ts">
  import {
    SvelteFlow,
    Background,
    Controls,
    Position,
    MarkerType,
    type Node,
    type Edge,
    type OnConnectEnd,
  } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
  import type { NodeRecord } from '$lib/types';
  import { layoutFlow, flowPosition } from './flow';
  import FlowBranchEdge from './FlowBranchEdge.svelte';
  const edgeTypes = { branch: FlowBranchEdge };
  type Point = { x: number; y: number };
  let {
    records,
    selectedId = '',
    editable = false,
    label = '话术流程图',
    onselect,
    onpositions,
    onconnect,
    onblankconnect,
    onedge,
  }: {
    records: NodeRecord[];
    selectedId?: string;
    editable?: boolean;
    label?: string;
    onselect?: (id: string) => void;
    onpositions?: (positions: { id: string; position: Point }[]) => void;
    onconnect?: (source: string, target: string) => void;
    onblankconnect?: (source: string, position: Point) => void;
    onedge?: (source: string, branchId: string) => void;
  } = $props();
  let nodes = $state<Node[]>([]);
  let edges = $state<Edge[]>([]);
  let viewport = $state({ x: 0, y: 0, zoom: 1 });
  let container: HTMLDivElement;
  $effect(() => {
    const flows = records.filter((r) => r.node.kind !== 'quick');
    const layout = layoutFlow(flows);
    const ids = new Set(flows.map((r) => r.node.id));
    nodes = flows.map(({ node, source }) => ({
      id: node.id,
      position: flowPosition(node, layout),
      data: {
        label: `${node.isEntry ? '起点 · ' : ''}${node.title}${source === 'personal' ? ' · 个人' : ''}`,
      },
      ariaLabel: node.title,
      selected: node.id === selectedId,
      sourcePosition: Position.Right,
      targetPosition: Position.Left,
    }));
    edges = flows.flatMap(({ node }) =>
      node.branches
        .filter((b) => ids.has(b.targetId))
        .map((b) => ({
          id: `${node.id}:${b.id}`,
          source: node.id,
          target: b.targetId,
          data: {
            branchId: b.id,
            onEdit: editable ? () => onedge?.(node.id, b.id) : undefined,
          },
          label: b.label,
          markerEnd: { type: MarkerType.ArrowClosed },
          type: 'branch',
        })),
    );
  });
  const endConnection: OnConnectEnd = (event, state) => {
    if (
      !editable ||
      state.isValid ||
      state.toNode ||
      state.fromHandle?.type !== 'source' ||
      !state.fromNode
    )
      return;
    const point = 'changedTouches' in event ? event.changedTouches[0] : event;
    if (!point) return;
    const bounds = container.getBoundingClientRect();
    if (
      point.clientX < bounds.left ||
      point.clientX > bounds.right ||
      point.clientY < bounds.top ||
      point.clientY > bounds.bottom
    )
      return;
    onblankconnect?.(state.fromNode.id, {
      x: (point.clientX - bounds.left - viewport.x) / viewport.zoom,
      y: (point.clientY - bounds.top - viewport.y) / viewport.zoom - 30,
    });
  };
</script>

<div
  class="flow-canvas"
  class:editable
  bind:this={container}
  role="region"
  aria-label={label}
>
  <SvelteFlow
    bind:nodes
    bind:edges
    bind:viewport
    {edgeTypes}
    fitView
    minZoom={0.15}
    maxZoom={2}
    nodesDraggable={editable}
    nodesConnectable={editable}
    elementsSelectable={true}
    deleteKey={null}
    onnodeclick={({ node }) => onselect?.(node.id)}
    onnodedragstop={({ nodes: moved }) =>
      onpositions?.(moved.map((n) => ({ id: n.id, position: n.position })))}
    onconnect={(connection) =>
      onconnect?.(connection.source, connection.target)}
    onconnectend={endConnection}
    onedgeclick={({ edge }) =>
      editable && onedge?.(edge.source, String(edge.data?.branchId || ''))}
    ><Background gap={22} /><Controls showLock={false} /></SvelteFlow
  >
</div>
