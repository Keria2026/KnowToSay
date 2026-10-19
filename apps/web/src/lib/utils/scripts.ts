import type { ScriptNode, ScriptKind, Material, NodeRecord } from '$lib/types';

export function replyPreview(
  node: ScriptNode,
  language: string,
  values: Record<string, string>,
) {
  const template = node.replies[language];
  if (!template?.trim())
    return { text: '', missing: [], languageMissing: true, ready: false };
  const missing = new Set<string>();
  for (const variable of node.variables)
    if (variable.required && !values[variable.name]?.trim())
      missing.add(variable.label || variable.name);
  const text = template.replace(/\{([^{}]+)\}/g, (_match, name: string) => {
    const value = values[name]?.trim();
    if (!value) {
      const variable = node.variables.find((v) => v.name === name);
      if (variable && !variable.required) return '';
      missing.add(variable?.label || name);
      return `{${name}}`;
    }
    return value;
  });
  return {
    text,
    missing: [...missing],
    languageMissing: false,
    ready: missing.size === 0,
  };
}

export function newNode(
  kind: ScriptKind = 'flow',
  flowId: string | null = null,
): ScriptNode {
  return {
    id: crypto.randomUUID(),
    title: '新话术',
    question: '',
    category: kind === 'quick' ? '未分类' : '',
    kind,
    flowId: kind === 'flow' ? flowId : null,
    isEntry: false,
    replies: { 'zh-CN': '', ko: '' },
    variables: [],
    branches: [],
    materialIds: [],
    position: null,
  };
}

export function searchLibrary(materials: Material[], query: string) {
  const q = query.trim().toLocaleLowerCase();
  return materials.flatMap((material) => {
    const direct = [
      material.name,
      material.category,
      material.notes,
      material.text,
      ...material.tags,
    ].some((s) => s.toLocaleLowerCase().includes(q));
    const rows = q
      ? material.rows
          .filter((row) =>
            row.cells.some((cell) => cell.toLocaleLowerCase().includes(q)),
          )
          .slice(0, 15)
      : [];
    return direct || rows.length ? [{ material, rows }] : [];
  });
}
export function searchScripts(nodes: NodeRecord[], query: string) {
  const q = query.trim().toLocaleLowerCase();
  return nodes.filter(({ node }) =>
    [
      node.title,
      node.question,
      node.kind === 'quick' ? node.category : '',
      ...Object.values(node.replies),
    ].some((s) => s.toLocaleLowerCase().includes(q)),
  );
}

export function bytes(value: number) {
  if (!value) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.min(4, Math.floor(Math.log(value) / Math.log(1024)));
  return `${(value / 1024 ** i).toFixed(i < 2 ? 0 : 1)} ${units[i]}`;
}
export const languageName = (code: string) =>
  ({
    'zh-CN': '简体中文',
    ko: '한국어 · 韩语',
    en: 'English · 英语',
    ja: '日本語 · 日语',
  })[code] || code;
export const kindName = (kind: string) =>
  ({ text: '文字', image: '图片', video: '视频', excel: '表格', file: '文件' })[
    kind
  ] || '文件';

export const scriptKindName = (kind: ScriptNode['kind']) =>
  kind === 'quick' ? '快捷话术' : '流程话术';

export const quickCategoryName = (node: Pick<ScriptNode, 'category'>) =>
  node.category.trim() || '未分类';
