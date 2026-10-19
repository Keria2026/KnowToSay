import type { Flow } from '$lib/types';

export const flowOrderKey = (groupId: string) =>
  `knowtosay.flow-order.${groupId}`;

export function readFlowOrder(value: string | null): string[] {
  try {
    const parsed: unknown = JSON.parse(value || '[]');
    return Array.isArray(parsed)
      ? parsed.filter((id): id is string => typeof id === 'string')
      : [];
  } catch {
    return [];
  }
}

export function orderedFlowIds(flows: Flow[], preferred: string[]): string[] {
  const available = new Set(flows.map((flow) => flow.id));
  return [
    ...new Set([...preferred.filter((id) => available.has(id)), ...available]),
  ];
}

export function orderFlows(flows: Flow[], preferred: string[]): Flow[] {
  const byId = new Map(flows.map((flow) => [flow.id, flow]));
  return orderedFlowIds(flows, preferred).map((id) => byId.get(id)!);
}
