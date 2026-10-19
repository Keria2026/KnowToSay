import { describe, it, expect } from 'bun:test';
import type { NodeRecord } from '../../types';
import { layoutFlow, flowNodeHeight, flowPosition } from './flow';
const record = (
  id: string,
  targets: string[] = [],
  kind: 'flow' | 'quick' = 'flow',
): NodeRecord => ({
  node: {
    id,
    title: id,
    question: '',
    category: '',
    kind,
    flowId: kind === 'flow' ? 'one' : null,
    isEntry: id === 'start',
    replies: {},
    variables: [],
    materialIds: [],
    branches: targets.map((targetId) => ({
      id: `${id}-${targetId}`,
      label: targetId,
      targetId,
    })),
  },
  source: 'official',
  official: null,
});
describe('分支归属与自动排列', () => {
  it('为各分支保留完整子树空间，不把第一条分支的下一步排到第二条分支下面', () => {
    const records = [
      record('start', ['small', 'early', 'other']),
      record('small', ['small-next']),
      record('small-next', ['small-end']),
      record('small-end'),
      record('early', ['early-next']),
      record('early-next'),
      record('other'),
    ];
    const layout = layoutFlow(records);
    const bottom = (id: string) =>
      layout.get(id)!.y +
      flowNodeHeight(records.find((r) => r.node.id === id)!.node);
    expect(bottom('small-next')).toBeLessThan(layout.get('early')!.y);
    expect(bottom('small-end')).toBeLessThan(layout.get('early-next')!.y);
    expect(bottom('early-next')).toBeLessThan(layout.get('other')!.y);
    expect([...layoutFlow([...records].reverse())].sort()).toEqual(
      [...layout].sort(),
    );
  });
  it('汇合必须在最长上游路径之后，同一列的节点和分支文本不重叠', () => {
    const records = [
      record('start', ['short', 'long']),
      record('short', ['merge']),
      record('long', ['middle']),
      record('middle', ['merge']),
      record('merge', ['end']),
      record('end'),
    ];
    const layout = layoutFlow(records);
    expect(layout.get('merge')!.x).toBeGreaterThan(layout.get('middle')!.x);
    expect(layout.get('end')!.x).toBeGreaterThan(layout.get('merge')!.x);
    for (const a of records)
      for (const b of records) {
        if (a.node.id === b.node.id) continue;
        const p = layout.get(a.node.id)!,
          q = layout.get(b.node.id)!;
        if (p.x === q.x && p.y < q.y)
          expect(p.y + flowNodeHeight(a.node)).toBeLessThan(q.y);
      }
  });
  it('循环、断开的循环和失效引用不会死循环，手动位置不会被隐式覆盖', () => {
    const records = [
      record('start', ['b', 'missing']),
      record('b', ['start']),
      record('c', ['d']),
      record('d', ['c']),
      record('faq', [], 'quick'),
    ];
    const layout = layoutFlow(records);
    expect(layout.size).toBe(4);
    expect(new Set([...layout.values()].map((p) => `${p.x},${p.y}`)).size).toBe(
      4,
    );
    expect(layout.has('missing')).toBe(false);
    const node = { ...records[0].node, position: { x: -777, y: 999 } };
    expect(flowPosition(node, layout)).toEqual(node.position!);
  });
  it('2000 个连续步骤可直接排列', () => {
    const records = Array.from({ length: 2000 }, (_, i) =>
      record(String(i), i < 1999 ? [String(i + 1)] : []),
    );
    const layout = layoutFlow(records);
    expect(layout.size).toBe(2000);
    expect(layout.get('1999')!.x).toBe(1999 * 420);
  });
});
