import type { Folder, Material } from '$lib/types';
export function folderPath(
  folders: Folder[],
  id: string | null | undefined,
): string {
  const names: string[] = [];
  const seen = new Set<string>();
  while (id && !seen.has(id)) {
    seen.add(id);
    const folder = folders.find((f) => f.id === id);
    if (!folder) break;
    names.unshift(folder.name);
    id = folder.parentId;
  }
  return names.join(' / ') || '资料根目录';
}
export function folderOf(material: Material, folders: Folder[]) {
  return (
    material.folderId ??
    folders.find((f) => !f.parentId && f.name === material.category)?.id ??
    null
  );
}
export function folderDescendants(folders: Folder[], id: string) {
  const ids = new Set([id]);
  let changed = true;
  while (changed) {
    changed = false;
    for (const folder of folders)
      if (folder.parentId && ids.has(folder.parentId) && !ids.has(folder.id)) {
        ids.add(folder.id);
        changed = true;
      }
  }
  return ids;
}
export function libraryItems(materials: Material[]) {
  const images = new Set(
    materials.filter((m) => m.kind === 'case').flatMap((m) => m.imageIds ?? []),
  );
  return materials.filter((m) => !images.has(m.id));
}
