export const groupOrderKey = (
  groupId: string,
  kind: 'flow' | 'quick' = 'flow',
) =>
  `knowtosay.${kind === 'flow' ? 'flow' : 'quick-category'}-order.${groupId}`;

export function readGroupOrder(value: string | null): string[] {
  try {
    const parsed: unknown = JSON.parse(value || '[]');
    return Array.isArray(parsed)
      ? parsed.filter((id): id is string => typeof id === 'string')
      : [];
  } catch {
    return [];
  }
}

export function orderedGroupIds(
  flows: { id: string }[],
  preferred: string[],
): string[] {
  const available = new Set(flows.map((flow) => flow.id));
  return [
    ...new Set([...preferred.filter((id) => available.has(id)), ...available]),
  ];
}

export function orderGroups<T extends { id: string }>(
  flows: T[],
  preferred: string[],
): T[] {
  const byId = new Map(flows.map((flow) => [flow.id, flow]));
  return orderedGroupIds(flows, preferred).map((id) => byId.get(id)!);
}
