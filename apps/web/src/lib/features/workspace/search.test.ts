import { describe, it, expect } from 'bun:test';
import { globalScope, scopedScripts, scopedMaterials } from './search';
import { newNode } from '$lib/utils/scripts';
import { shortcutFromKey } from './shortcut';
import { columnName } from '../knowledge/sheet';

describe('销售搜索与操作', () => {
  it('默认查全库，可限定流程、快捷分类和知识库分类', () => {
    const flow = {
      ...newNode('flow', 'flow-a'),
      id: 'flow',
      category: '不参与流程分类',
    };
    const other = { ...newNode('flow', 'flow-b'), id: 'other' };
    const quick = { ...newNode('quick'), id: 'quick', category: '价格咨询' };
    const nodes = [flow, other, quick].map((node) => ({
      node,
      source: 'draft',
      official: null,
    }));
    expect(scopedScripts(nodes, globalScope()).length).toBe(3);
    expect(
      scopedScripts(nodes, { area: 'flow', group: 'flow-a' }).map(
        (r) => r.node.id,
      ),
    ).toEqual(['flow']);
    expect(
      scopedScripts(nodes, { area: 'quick', group: '价格咨询' }).map(
        (r) => r.node.id,
      ),
    ).toEqual(['quick']);
    expect(scopedScripts(nodes, { area: 'library', group: '' })).toEqual([]);
    const material = {
      id: 'm',
      name: '文案',
      kind: 'text',
      category: '',
      tags: [],
      notes: '',
      text: '正文',
      hash: null,
      size: 0,
      mime: 'text/plain',
      rows: [],
    };
    const hits = [{ material, rows: [] }];
    expect(scopedMaterials(hits, { area: 'flow', group: '' })).toEqual([]);
    expect(scopedMaterials(hits, { area: 'library', group: '未分类' })).toEqual(
      hits,
    );
    expect(scopedMaterials(hits, { area: 'library', group: '售后' })).toEqual(
      [],
    );
  });
  it('快捷键录入按物理按键识别，避免输入法内容或普通字母成为全局快捷键', () => {
    const event = {
      key: ' ',
      code: 'Space',
      ctrlKey: true,
      altKey: true,
      shiftKey: false,
      metaKey: false,
    };
    expect(shortcutFromKey(event)).toBe('Ctrl+Alt+Space');
    expect(
      shortcutFromKey({
        ...event,
        key: '搜',
        code: 'KeyK',
        altKey: false,
        shiftKey: true,
      }),
    ).toBe('Ctrl+Shift+K');
    expect(
      shortcutFromKey({
        ...event,
        key: 'k',
        code: 'KeyK',
        ctrlKey: false,
        altKey: false,
      }),
    ).toBeNull();
    expect(
      shortcutFromKey({ ...event, key: 'Control', code: 'ControlLeft' }),
    ).toBeNull();
  });
  it('表格列地址跨 Z 和 AZ 后仍然正确', () => {
    expect([0, 25, 26, 51, 52, 701, 702].map(columnName)).toEqual([
      'A',
      'Z',
      'AA',
      'AZ',
      'BA',
      'ZZ',
      'AAA',
    ]);
  });
});
