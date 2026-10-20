import { invoke, isTauri, convertFileSrc } from '@tauri-apps/api/core';
import type { Material } from '$lib/types';

export const native = () => isTauri();
export async function api<T>(
  path: string,
  method = 'GET',
  payload?: unknown,
): Promise<T> {
  try {
    if (native())
      return await invoke<T>('api_request', {
        method,
        path: `/api/v1${path}`,
        payload: payload ?? null,
      });
    const response = await fetch(`/api/v1${path}`, {
      method,
      headers: { 'content-type': 'application/json', 'x-knowtosay-local': '1' },
      body: payload === undefined ? undefined : JSON.stringify(payload),
    });
    const value = await response.json().catch(() => null);
    if (!response.ok)
      throw value || new Error('本地服务未响应，请检查开发服务是否启动');
    return value as T;
  } catch (error) {
    if (error instanceof Error) throw error;
    if (typeof error === 'object' && error && 'message' in error)
      throw new Error(String(error.message));
    throw new Error(String(error));
  }
}

export async function copyText(text: string) {
  if (native()) {
    const { writeText } = await import('@tauri-apps/plugin-clipboard-manager');
    await writeText(text);
  } else await navigator.clipboard.writeText(text);
}
export async function materialUrl(material: Material) {
  if (!material.hash) return '';
  if (!native())
    return `/api/v1/materials/${encodeURIComponent(material.id)}/file`;
  return convertFileSrc(
    await invoke<string>('material_path', { id: material.id }),
  );
}
export async function copyMaterial(material: Material) {
  if (material.kind !== 'image') return copyText(material.text);
  const response = await fetch(await materialUrl(material));
  if (!response.ok) throw new Error('图片读取失败，请导出后使用');
  const bitmap = await createImageBitmap(await response.blob());
  try {
    const canvas = document.createElement('canvas');
    canvas.width = bitmap.width;
    canvas.height = bitmap.height;
    const context = canvas.getContext('2d');
    if (!context) throw new Error('无法处理图片，请导出后使用');
    context.drawImage(bitmap, 0, 0);
    if (native()) {
      const { Image } = await import('@tauri-apps/api/image');
      const { writeImage } =
        await import('@tauri-apps/plugin-clipboard-manager');
      const data = context.getImageData(0, 0, canvas.width, canvas.height);
      const image = await Image.new(
        new Uint8Array(data.data.buffer),
        canvas.width,
        canvas.height,
      );
      try {
        await writeImage(image);
      } finally {
        await image.close();
      }
    } else {
      const png = await new Promise<Blob>((resolve, reject) =>
        canvas.toBlob(
          (blob) => (blob ? resolve(blob) : reject(new Error('图片转换失败'))),
          'image/png',
        ),
      );
      await navigator.clipboard.write([
        new ClipboardItem({ 'image/png': png }),
      ]);
    }
  } finally {
    bitmap.close();
  }
}
export async function exportMaterial(material: Material) {
  if (!material.hash) return copyText(material.text);
  if (native()) {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const destination = await save({ defaultPath: material.name });
    if (destination)
      await invoke('export_material', { id: material.id, destination });
    return !!destination;
  }
  const a = document.createElement('a');
  a.href = `/api/v1/materials/${encodeURIComponent(material.id)}/file?download=true`;
  a.download = material.name;
  a.click();
  return true;
}
export async function revealMaterial(id: string) {
  await invoke('reveal_material', { id });
}
export async function chooseDirectory() {
  if (!native()) return null;
  const { open } = await import('@tauri-apps/plugin-dialog');
  const result = await open({ directory: true, multiple: false });
  return typeof result === 'string' ? result : null;
}
export async function uploadFiles(files: FileList | File[]) {
  const data = new FormData();
  for (const file of Array.from(files)) data.append('files', file);
  const response = await fetch('/api/v1/materials/upload', {
    method: 'POST',
    headers: { 'x-knowtosay-local': '1' },
    body: data,
  });
  const value = await response.json();
  if (!response.ok) throw new Error(value.message || '导入失败');
  return value as Material[];
}
export async function selectNativeFiles(imagesOnly = false) {
  const { open } = await import('@tauri-apps/plugin-dialog');
  const paths = await open({
    multiple: true,
    directory: false,
    filters: imagesOnly
      ? [
          {
            name: '图片',
            extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp'],
          },
        ]
      : undefined,
  });
  if (!paths) return [];
  return invoke<Material[]>('import_files', {
    paths: Array.isArray(paths) ? paths : [paths],
  });
}
