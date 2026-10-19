import type { NodeRecord, ScriptNode } from '$lib/types';

export function layoutFlow(records: NodeRecord[]) {
  const flows = records.filter((r) => r.node.kind !== 'quick');
  const byId = new Map(flows.map((r) => [r.node.id, r.node]));
  const incoming = new Set(
    flows.flatMap((r) => r.node.branches.map((b) => b.targetId)),
  );
  const positions = new Map<string, { x: number; y: number }>();
  const rows = new Map<number, number>();
  const queue: { id: string; depth: number }[] = [];
  const enqueue = (id: string, depth: number) => {
    if (!byId.has(id) || positions.has(id)) return;
    const row = rows.get(depth) || 0;
    positions.set(id, { x: depth * 300, y: row * 160 });
    rows.set(depth, row + 1);
    queue.push({ id, depth });
  };
  const visit = () => {
    for (let i = 0; i < queue.length; i++) {
      const { id, depth } = queue[i];
      for (const branch of byId.get(id)!.branches)
        enqueue(branch.targetId, depth + 1);
    }
    queue.length = 0;
  };
  for (const { node } of flows)
    if (node.isEntry || !incoming.has(node.id)) enqueue(node.id, 0);
  visit();
  // A disconnected cycle may have neither an entry nor a root.
  for (const { node } of flows) {
    if (!positions.has(node.id)) {
      enqueue(node.id, 0);
      visit();
    }
  }
  return positions;
}

export function connectedFlow(records: NodeRecord[], selectedId: string) {
  const flows = records.filter((r) => r.node.kind !== 'quick');
  const neighbors = new Map(flows.map((r) => [r.node.id, new Set<string>()]));
  for (const { node } of flows)
    for (const branch of node.branches) {
      if (!neighbors.has(branch.targetId)) continue;
      neighbors.get(node.id)!.add(branch.targetId);
      neighbors.get(branch.targetId)!.add(node.id);
    }
  if (!neighbors.has(selectedId)) return [];
  const seen = new Set([selectedId]);
  const pending = [selectedId];
  for (let i = 0; i < pending.length; i++)
    for (const id of neighbors.get(pending[i])!) {
      if (!seen.has(id)) {
        seen.add(id);
        pending.push(id);
      }
    }
  return flows.filter((r) => seen.has(r.node.id));
}

export function flowPosition(
  node: ScriptNode,
  layout: ReturnType<typeof layoutFlow>,
) {
  return node.position || layout.get(node.id) || { x: 0, y: 0 };
}
