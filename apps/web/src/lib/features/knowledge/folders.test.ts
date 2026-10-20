import { describe, expect, it } from 'bun:test';
import { folderPath, folderDescendants, libraryItems } from './folders';
import { scopedMaterials } from '../workspace/search';
import type { Material } from '$lib/types';

describe('文件夹与文案多图案例', () => {
  const folders = [
    { id: 'parent', name: '案例', parentId: null },
    { id: 'child', name: '治疗', parentId: 'parent' },
    { id: 'other', name: '其他', parentId: null },
  ];
  it('按身份组成路径及子树，文件夹范围包含子级但不包含其他目录', () => {
    expect(folderPath(folders, 'child')).toBe('案例 / 治疗');
    expect([...folderDescendants(folders, 'parent')]).toEqual([
      'parent',
      'child',
    ]);
    const base: Material = {
      id: 'one',
      name: '案例',
      kind: 'case',
      category: '',
      folderId: 'child',
      imageIds: ['image'],
      text: '文案',
      notes: '',
      tags: [],
      hash: null,
      size: 0,
      mime: 'text/plain',
      rows: [],
    };
    const hits = [
      { material: base, rows: [] },
      { material: { ...base, id: 'two', folderId: 'other' }, rows: [] },
    ];
    expect(
      scopedMaterials(hits, { area: 'library', group: 'parent' }, folders).map(
        (h) => h.material.id,
      ),
    ).toEqual(['one']);
    expect(
      libraryItems([
        base,
        { ...base, id: 'image', kind: 'image', imageIds: [] },
      ]).map((m) => m.id),
    ).toEqual(['one']);
  });
});
