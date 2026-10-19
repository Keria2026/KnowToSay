import { describe, it, expect } from 'bun:test';
import type { NodeRecord } from '../../types';
import { connectedFlow, layoutFlow } from './flow';

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

describe('完整流程与自动排版', () => {
  it('从流程中间展开上游、同级分支和汇合，排除其他流程与快捷话术', () => {
    const records = [
      record('start', ['yes', 'no']),
      record('yes', ['end']),
      record('no', ['end']),
      record('end'),
      record('other'),
      record('faq', [], 'quick'),
    ];
    expect(connectedFlow(records, 'yes').map((r) => r.node.id)).toEqual([
      'start',
      'yes',
      'no',
      'end',
    ]);
    expect(connectedFlow(records, 'faq')).toEqual([]);
  });
  it('循环、断开的循环和失效引用不会死循环或生成重复位置', () => {
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
    expect(connectedFlow(records, 'd').map((r) => r.node.id)).toEqual([
      'c',
      'd',
    ]);
  });
});
