import { describe, it, expect } from 'bun:test';
import {
  replyPreview,
  replyVariables,
  searchLibrary,
  searchScripts,
  matchingReply,
} from './scripts';
import type { ScriptNode, Material } from '../types';
const node: ScriptNode = {
  id: 'one',
  title: '测试',
  question: '',
  category: '',
  isEntry: true,
  replies: { ko: '안녕하세요, {name}. 가격은 {price}입니다.', 'zh-CN': '您好' },
  variables: [
    { name: 'name', label: '客户姓名', required: true },
    { name: 'price', label: '价格', required: true },
  ],
  branches: [],
  materialIds: [],
};
describe('销售复制前校验', () => {
  it('缺少目标语言时不回退到其他语言', () => {
    const r = replyPreview(node, 'en', {});
    expect(r.languageMissing).toBe(true);
    expect(r.ready).toBe(false);
    expect(r.text).toBe('');
  });
  it('未填写变量时不可复制，已填写的值不再被当作模板执行', () => {
    expect(replyPreview(node, 'ko', { name: '客户' }).missing).toEqual([
      '价格',
    ]);
    const r = replyPreview(node, 'ko', {
      name: '{另一个变量}',
      price: '₩20,000',
    });
    expect(r.ready).toBe(true);
    expect(r.text).toBe('안녕하세요, {另一个变量}. 가격은 ₩20,000입니다.');
  });
  it('未声明的占位符仍然阻止原样误发', () => {
    const r = replyPreview({ ...node, variables: [] }, 'ko', {});
    expect(r.ready).toBe(false);
    expect(r.missing).toContain('name');
  });
  it('明确声明的可选变量可以留空', () => {
    const r = replyPreview(
      {
        ...node,
        variables: node.variables.map((v) => ({ ...v, required: false })),
      },
      'ko',
      {},
    );
    expect(r.ready).toBe(true);
    expect(r.text).not.toContain('{');
  });
});
describe('知识库表格搜索', () => {
  it('搜索单元格并保留工作表和实际行号', () => {
    const m: Material = {
      id: 'm',
      name: '价格表',
      kind: 'excel',
      category: '产品',
      tags: [],
      notes: '',
      text: '',
      mime: 'application/octet-stream',
      hash: null,
      size: 10,
      rows: [
        { sheet: '韩国区', row: 19, cells: ['SKU-237', '₩20,000'] },
        { sheet: '韩国区', row: 20, cells: ['另一个产品'] },
      ],
    };
    const results = searchLibrary([m], 'sku-237');
    expect(results).toHaveLength(1);
    expect(results[0].rows[0].sheet).toBe('韩国区');
    expect(results[0].rows[0].row).toBe(19);
    expect(searchLibrary([m], '不存在')).toHaveLength(0);
  });
});

describe('快捷话术多回复', () => {
  const multi: ScriptNode = {
    ...node,
    kind: 'quick',
    replyOptions: [
      {
        id: 'reassure',
        name: '安抚客户',
        replies: { 'zh-CN': '可以先了解情况', ko: '먼저 알아보세요' },
      },
      { id: 'ask', name: '询问产品', replies: { ko: '{产品名} 안내' } },
    ],
  };
  it('只复制所选回复，不拼接默认内容或回退语言', () => {
    expect(replyPreview(multi, 'ko', {}, 'reassure')).toMatchObject({
      ready: true,
      text: '먼저 알아보세요',
    });
    expect(replyPreview(multi, 'zh-CN', {}, 'ask')).toMatchObject({
      ready: false,
      languageMissing: true,
      text: '',
    });
    expect(replyPreview(multi, 'ko', {}, 'removed')).toMatchObject({
      ready: false,
      languageMissing: true,
    });
  });
  it('每组按自己的占位符校验变量，搜索可命中其他回复', () => {
    expect(replyVariables(multi, 'ko', 'ask').map((v) => v.name)).toEqual([
      '产品名',
    ]);
    expect(replyPreview(multi, 'ko', {}, 'ask').missing).toEqual(['产品名']);
    expect(replyPreview(multi, 'ko', { 产品名: '크림' }, 'ask').text).toBe(
      '크림 안내',
    );
    expect(
      searchScripts([{ node: multi, source: 'draft', official: null }], '먼저'),
    ).toHaveLength(1);
    expect(matchingReply(multi, '먼저')).toBe('reassure');
  });
});
