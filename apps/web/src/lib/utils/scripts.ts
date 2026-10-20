import type { ScriptNode, ScriptKind, Material, NodeRecord } from '$lib/types';

export function replyPreview(
  node: ScriptNode,
  language: string,
  values: Record<string, string>,
  replyId = '',
) {
  const template = repliesFor(node, replyId)[language];
  if (!template?.trim())
    return { text: '', missing: [], languageMissing: true, ready: false };
  const missing = new Set<string>();
  for (const variable of replyVariables(node, language, replyId))
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

export function replyVariables(
  node: ScriptNode,
  language: string,
  replyId = '',
) {
  return [
    ...new Map(
      [
        ...node.variables.filter(
          (v) =>
            !node.replyOptions?.length ||
            (repliesFor(node, replyId)[language] ?? '').includes(`{${v.name}}`),
        ),
        ...[
          ...(repliesFor(node, replyId)[language] || '').matchAll(
            /\{([^{}]+)\}/g,
          ),
        ].map(
          (m) =>
            node.variables.find((v) => v.name === m[1]) || {
              name: m[1],
              label: m[1],
              required: true,
            },
        ),
      ].map((v) => [v.name, v]),
    ).values(),
  ];
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
    replyOptions: [],
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
      ...replyChoices(node).flatMap((reply) => [
        reply.name,
        ...Object.values(reply.replies),
      ]),
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
  ({
    case: '案例',
    text: '文字',
    image: '图片',
    video: '视频',
    excel: '表格',
    file: '文件',
  })[kind] || '文件';

export const scriptKindName = (kind: ScriptNode['kind']) =>
  kind === 'quick' ? '快捷话术' : '流程话术';

export const quickCategoryName = (node: Pick<ScriptNode, 'category'>) =>
  node.category.trim() || '未分类';

export function replyChoices(node: ScriptNode) {
  return [
    { id: '', name: '默认回复', replies: node.replies },
    ...(node.replyOptions ?? []),
  ];
}
export function repliesFor(node: ScriptNode, replyId = '') {
  return replyId
    ? (node.replyOptions?.find((reply) => reply.id === replyId)?.replies ?? {})
    : node.replies;
}
export function matchingReply(node: ScriptNode, query: string) {
  const q = query.trim().toLocaleLowerCase();
  return (
    replyChoices(node).find((reply) =>
      [reply.name, ...Object.values(reply.replies)].some((text) =>
        text.toLocaleLowerCase().includes(q),
      ),
    )?.id ?? ''
  );
}
