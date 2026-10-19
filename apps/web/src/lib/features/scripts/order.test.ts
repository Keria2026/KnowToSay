import { describe, expect, it } from 'bun:test';
import { orderGroups, readGroupOrder } from './order';

describe('本机流程顺序', () => {
  it('保留用户顺序，追加新流程，忽略重复和已移除的流程', () => {
    const flows = [
      { id: 'a', name: '咨询' },
      { id: 'b', name: '复购' },
      { id: 'new', name: '新增流程' },
    ];
    const ordered = orderGroups(flows, ['b', 'removed', 'a', 'b']);
    expect(ordered.map((flow) => flow.id)).toEqual(['b', 'a', 'new']);
    expect(flows.map((flow) => flow.id)).toEqual(['a', 'b', 'new']);
    expect(
      orderGroups(
        flows.map((flow) => ({ ...flow, name: `${flow.name}已改名` })),
        ['b', 'a'],
      ).map((flow) => flow.id),
    ).toEqual(['b', 'a', 'new']);
  });
  it('本机偏好损坏时仍可展示现有流程', () => {
    for (const value of [null, '{invalid', '{}', 'null'])
      expect(readGroupOrder(value)).toEqual([]);
    expect(readGroupOrder('["a",null,17,"b"]')).toEqual(['a', 'b']);
  });
});
