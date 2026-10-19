<script lang="ts">
  import { Handle, Position, type Node, type NodeProps } from '@xyflow/svelte';
  import type { ScriptNode } from '$lib/types';
  type StepNode = Node<
    {
      node: ScriptNode;
      label: string;
      editable: boolean;
      onEdit?: (id: string) => void;
    },
    'step'
  >;
  let { data }: NodeProps<StepNode> = $props();
</script>

<div class="flow-step">
  <Handle id="in" type="target" position={Position.Left} style="top: 28px" />
  <div class="flow-step-title">{data.label}</div>
  {#each data.node.branches as branch, i (branch.id)}
    <div class="flow-step-branch">
      {#if data.editable}<button
          class="nodrag nopan"
          aria-label={`编辑分支：${branch.label}`}
          onclick={(event) => {
            event.stopPropagation();
            data.onEdit?.(branch.id);
          }}>{branch.label || '待填写条件'}</button
        >
      {:else}<span>{branch.label || '待填写条件'}</span>{/if}
    </div>
    <Handle
      id={branch.id}
      type="source"
      position={Position.Right}
      isConnectable={false}
      style={`top: ${78 + i * 44}px`}
    />
  {/each}
  <div class="flow-step-footer">
    {data.editable
      ? '拉箭头添加分支'
      : data.node.branches.length
        ? '按客户回答选择分支'
        : '当前流程到此处'}
  </div>
  {#if data.editable}<Handle
      id="new"
      type="source"
      position={Position.Right}
      style={`top: ${78 + data.node.branches.length * 44}px`}
    />{/if}
</div>
