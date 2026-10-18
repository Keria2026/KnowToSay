import { invoke } from '@tauri-apps/api/core';
import { api, native, copyText } from './api';
import type { Snapshot, ScriptNode, NodeRecord, SyncStatus } from '$lib/types';

class WorkspaceStore {
  data = $state<Snapshot | null>(null);
  loading = $state(true);
  busy = $state('');
  error = $state('');
  notice = $state('');
  nav = $state('workbench');
  query = $state('');
  selectedId = $state('');
  selectedMaterialId = $state('');
  language = $state('ko');
  values = $state<Record<string, string>>({});
  history = $state<string[]>([]);
  compact = $state(false);
  progress = $state<SyncStatus | null>(null);
  editorId = $state('');
  editorDirty = $state(false);
  freshNode = $state<ScriptNode | null>(null);
  nodes = $derived(this.data?.nodes ?? []);
  materials = $derived(this.data?.materials ?? []);
  workspace = $derived(this.data?.workspace ?? null);
  admin = $derived(this.workspace?.role === 'admin');
  current = $derived(
    this.nodes.find((n) => n.node.id === this.selectedId) ?? null,
  );
  languages = $derived([
    ...new Set([
      'ko',
      'zh-CN',
      ...this.nodes.flatMap((n) => Object.keys(n.node.replies)),
    ]),
  ]);

  async refresh() {
    const data = await api<Snapshot>('/snapshot');
    this.data = data;
    if (
      !this.selectedId ||
      !data.nodes.some((n) => n.node.id === this.selectedId)
    )
      this.selectedId =
        data.nodes.find((n) => n.node.isEntry)?.node.id ||
        data.nodes[0]?.node.id ||
        '';
  }
  async start() {
    try {
      await this.refresh();
      this.error = '';
    } catch (error) {
      this.error = String(error);
    } finally {
      this.loading = false;
    }
  }
  async task<T>(
    label: string,
    action: () => Promise<T>,
    success = '',
  ): Promise<T | undefined> {
    if (this.busy) return undefined;
    this.busy = label;
    this.error = '';
    this.notice = '';
    try {
      const result = await action();
      if (success) this.notify(success);
      return result;
    } catch (error) {
      this.error = error instanceof Error ? error.message : String(error);
      return undefined;
    } finally {
      this.busy = '';
    }
  }
  notify(text: string) {
    this.notice = text;
    setTimeout(() => {
      if (this.notice === text) this.notice = '';
    }, 4500);
  }
  navigate(nav: string) {
    if (this.editorDirty && this.nav === 'scripts') {
      if (!confirm('当前话术尚未保存，放弃修改并离开？')) return false;
      this.editorDirty = false;
      this.freshNode = null;
      this.editorId = '';
    }
    this.nav = nav;
    this.query = '';
    return true;
  }
  select(id: string, reset = false) {
    if (this.nav === 'scripts' && !this.navigate('workbench')) return;
    if (reset) this.history = [];
    else if (this.selectedId && this.selectedId !== id)
      this.history = [...this.history, this.selectedId];
    this.selectedId = id;
    this.query = '';
    this.nav = 'workbench';
  }
  back() {
    const previous = this.history.at(-1);
    if (previous) {
      this.selectedId = previous;
      this.history = this.history.slice(0, -1);
    }
  }
  async save(node: ScriptNode) {
    return this.task(
      '保存话术',
      async () => {
        this.data = await api<Snapshot>(
          `/scripts/${encodeURIComponent(node.id)}`,
          'PUT',
          node,
        );
        this.editorId = node.id;
        this.freshNode = null;
        this.selectedId = node.id;
      },
      this.admin
        ? '已保存为管理员草稿，发布后业务员才会收到'
        : '已保存个人版本，管理员更新不会覆盖此修改',
    );
  }
  edit(record?: NodeRecord) {
    this.editorId = record?.node.id || this.current?.node.id || '';
    this.freshNode = null;
    this.nav = 'scripts';
    this.query = '';
  }
  async reset(record: NodeRecord) {
    if (!confirm('放弃这条话术的个人修改，恢复当前管理员版本？')) return false;
    return (
      (await this.task(
        '恢复管理员版本',
        async () => {
          this.data = await api<Snapshot>(
            `/scripts/${encodeURIComponent(record.node.id)}/reset`,
            'POST',
          );
          return true;
        },
        '已恢复管理员版本',
      )) || false
    );
  }
  async copy(text: string) {
    await this.task('复制', () => copyText(text), '已复制，可粘贴到聊天窗口');
  }
  async toggleCompact() {
    await this.task('切换窗口', async () => {
      if (native()) await invoke('window_mode', { compact: !this.compact });
      this.compact = !this.compact;
    });
  }
}
export const app = new WorkspaceStore();
