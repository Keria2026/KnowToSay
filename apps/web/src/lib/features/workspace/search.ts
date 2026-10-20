import type { NodeRecord, MaterialSearchResult, Folder } from '$lib/types';
import { folderOf, folderDescendants } from '../knowledge/folders';
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
  folders: Folder[] = [],
) {
  return scope.area === 'all'
    ? hits
    : scope.area === 'library'
      ? hits.filter(
          ({ material }) =>
            !scope.group ||
            (folders.some((f) => f.id === scope.group)
              ? folderDescendants(folders, scope.group).has(
                  folderOf(material, folders) ?? '',
                )
              : (material.category.trim() || '未分类') === scope.group),
        )
      : [];
}
