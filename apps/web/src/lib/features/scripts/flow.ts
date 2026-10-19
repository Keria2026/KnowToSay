import type { NodeRecord, ScriptNode } from '$lib/types';

export const flowNodeHeight = (node: ScriptNode) =>
  100 + node.branches.length * 44;

/** Lay out the real graph, preserving branch order and space for each subtree.
 * DFS back edges are excluded from ranking, but remain visible as return arrows.
 * Iterative traversal also handles long imported flows without recursive overflow.
 */
export function layoutFlow(records: NodeRecord[]) {
  const steps = records
    .filter((r) => r.node.kind !== 'quick')
    .map((r) => r.node);
  const byId = new Map(steps.map((node) => [node.id, node]));
  const incoming = new Set(
    steps.flatMap((n) => n.branches.map((b) => b.targetId)),
  );
  const ordered = [...steps].sort(
    (a, b) =>
      Number(b.isEntry) - Number(a.isEntry) ||
      Number(incoming.has(a.id)) - Number(incoming.has(b.id)) ||
      a.id.localeCompare(b.id),
  );
  const state = new Map<string, number>();
  const children = new Map(steps.map((n) => [n.id, [] as string[]]));
  const forward = new Map(steps.map((n) => [n.id, new Set<string>()]));
  const parents = new Map(steps.map((n) => [n.id, new Set<string>()]));
  const roots: string[] = [];
  const postorder: string[] = [];
  for (const root of ordered) {
    if (state.has(root.id)) continue;
    roots.push(root.id);
    state.set(root.id, 1);
    const stack = [{ id: root.id, branch: 0 }];
    while (stack.length) {
      const frame = stack.at(-1)!;
      const branch = byId.get(frame.id)!.branches[frame.branch++];
      if (!branch) {
        state.set(frame.id, 2);
        postorder.push(frame.id);
        stack.pop();
        continue;
      }
      const target = branch.targetId;
      if (!byId.has(target) || state.get(target) === 1) continue;
      forward.get(frame.id)!.add(target);
      parents.get(target)!.add(frame.id);
      if (!state.has(target)) {
        children.get(frame.id)!.push(target);
        state.set(target, 1);
        stack.push({ id: target, branch: 0 });
      }
    }
  }
  // Longest-path ranks put a shared next step after all of its upstream steps.
  const rank = new Map(steps.map((n) => [n.id, 0]));
  const degree = new Map([...parents].map(([id, p]) => [id, p.size]));
  const queue = ordered.filter((n) => degree.get(n.id) === 0).map((n) => n.id);
  for (let i = 0; i < queue.length; i++) {
    const id = queue[i];
    for (const target of forward.get(id)!) {
      rank.set(target, Math.max(rank.get(target)!, rank.get(id)! + 1));
      degree.set(target, degree.get(target)! - 1);
      if (!degree.get(target)) queue.push(target);
    }
  }
  const gap = 80;
  const height = new Map<string, number>();
  for (const id of postorder) {
    const subtree = children.get(id)!;
    const total =
      subtree.reduce((sum, child) => sum + height.get(child)!, 0) +
      Math.max(0, subtree.length - 1) * gap;
    height.set(id, Math.max(flowNodeHeight(byId.get(id)!), total));
  }
  const positions = new Map<string, { x: number; y: number }>();
  let top = 0;
  for (const root of roots) {
    const stack = [{ id: root, top }];
    while (stack.length) {
      const { id, top } = stack.pop()!;
      const node = byId.get(id)!;
      positions.set(id, {
        x: rank.get(id)! * 420,
        y: top + (height.get(id)! - flowNodeHeight(node)) / 2,
      });
      const subtree = children.get(id)!;
      const total =
        subtree.reduce((sum, child) => sum + height.get(child)!, 0) +
        Math.max(0, subtree.length - 1) * gap;
      let childTop = top + (height.get(id)! - total) / 2;
      for (const child of subtree) {
        stack.push({ id: child, top: childTop });
        childTop += height.get(child)! + gap;
      }
    }
    top += height.get(root)! + gap;
  }
  // Center merges between their actual parents; keep nodes in each column apart.
  const columns = new Map<number, ScriptNode[]>();
  for (const node of steps) {
    const depth = rank.get(node.id)!;
    if (!columns.has(depth)) columns.set(depth, []);
    columns.get(depth)!.push(node);
  }
  for (const depth of [...columns.keys()].sort((a, b) => a - b)) {
    const column = columns.get(depth)!;
    for (const node of column) {
      const upstream = [...parents.get(node.id)!];
      if (upstream.length > 1) {
        positions.get(node.id)!.y =
          upstream.reduce(
            (sum, id) =>
              sum + positions.get(id)!.y + flowNodeHeight(byId.get(id)!) / 2,
            0,
          ) /
            upstream.length -
          flowNodeHeight(node) / 2;
      }
    }
    column.sort(
      (a, b) =>
        positions.get(a.id)!.y - positions.get(b.id)!.y ||
        a.id.localeCompare(b.id),
    );
    let bottom = -Infinity;
    for (const node of column) {
      const position = positions.get(node.id)!;
      position.y = Math.max(position.y, bottom);
      bottom = position.y + flowNodeHeight(node) + gap;
    }
  }
  return positions;
}

export function flowPosition(
  node: ScriptNode,
  layout: ReturnType<typeof layoutFlow>,
) {
  return node.position || layout.get(node.id) || { x: 0, y: 0 };
}
