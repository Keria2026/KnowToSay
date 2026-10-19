import type { NodeRecord, MaterialSearchResult } from '$lib/types';
import { quickCategoryName } from '$lib/utils/scripts';
export type SearchArea = 'all' | 'flow' | 'quick' | 'library';
export type SearchScope = { area: SearchArea; group: string };
export const globalScope = (): SearchScope => ({ area: 'all', group: '' });
export function scopedScripts(records: NodeRecord[], scope: SearchScope) {
  return records.filter(
    ({ node }) =>
      scope.area !== 'library' &&
      (scope.area === 'all' ||
        (scope.area === 'quick'
          ? node.kind === 'quick'
          : node.kind !== 'quick')) &&
      (!scope.group ||
        scope.area === 'all' ||
        (scope.area === 'quick'
          ? quickCategoryName(node) === scope.group
          : node.flowId === scope.group)),
  );
}
export function scopedMaterials(
  hits: MaterialSearchResult[],
  scope: SearchScope,
) {
  return scope.area === 'all'
    ? hits
    : scope.area === 'library'
      ? hits.filter(
          ({ material }) =>
            !scope.group ||
            (material.category.trim() || '未分类') === scope.group,
        )
      : [];
}
