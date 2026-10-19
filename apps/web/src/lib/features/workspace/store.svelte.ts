import { invoke } from '@tauri-apps/api/core';
import { api, native, copyText } from './api';
import {
  flowOrderKey,
  readFlowOrder,
  orderedFlowIds,
  orderFlows,
} from '../scripts/order';
import type {
  Snapshot,
  ScriptNode,
  ScriptKind,
  NodeRecord,
  SyncStatus,
} from '$lib/types';

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
  flowId = $state('');
  flowOrder = $state<string[]>([]);
  flows = $derived(orderFlows(this.data?.flows ?? [], this.flowOrder));
  activeFlowId = $derived(
    this.flows.some((f) => f.id === this.flowId)
      ? this.flowId
      : this.flows[0]?.id || '',
  );
  editorKind = $state<ScriptKind>('flow');
  browseKind = $state<ScriptKind>('flow');
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

  restoreFlowOrder() {
    if (!this.workspace) {
      this.flowOrder = [];
      return;
    }
    try {
      const key = flowOrderKey(this.workspace.groupId);
      const order = orderedFlowIds(
        this.data?.flows ?? [],
        readFlowOrder(localStorage.getItem(key)),
      );
      localStorage.setItem(key, JSON.stringify(order));
      this.flowOrder = order;
    } catch {
      this.flowOrder = [];
      this.error = '无法读取或保存本机流程顺序，请检查本机存储后重试';
    }
  }
  saveFlowOrder(ids: string[]) {
    if (!this.workspace) return false;
    this.error = '';
    try {
      const order = orderedFlowIds(this.data?.flows ?? [], ids);
      const selectedFlow = this.activeFlowId;
      localStorage.setItem(
        flowOrderKey(this.workspace.groupId),
        JSON.stringify(order),
      );
      this.flowId = selectedFlow;
      this.flowOrder = order;
      return true;
    } catch {
      this.error = '流程顺序保存失败，请检查本机存储后重试';
      return false;
    }
  }
  async refresh() {
    const data = await api<Snapshot>('/snapshot');
    this.data = data;
    if (
      !this.selectedId ||
      !data.nodes.some((n) => n.node.id === this.selectedId)
    ) {
      const candidates =
        this.flowId &&
        data.flows.some((f) => f.id === this.flowId) &&
        this.browseKind === 'flow'
          ? data.nodes.filter((r) => r.node.flowId === this.flowId)
          : data.nodes;
      this.selectedId =
        candidates.find((r) => r.node.isEntry)?.node.id ||
        candidates[0]?.node.id ||
        '';
      this.browseKind =
        data.nodes.find((n) => n.node.id === this.selectedId)?.node.kind ??
        'flow';
    }
    const selectedFlowId = this.current?.node.flowId;
    if (selectedFlowId) this.flowId = selectedFlowId;
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
      if (!confirm('当前话术与流程尚未保存，放弃全部修改并离开？'))
        return false;
      this.editorDirty = false;
      this.freshNode = null;
      this.editorId = '';
    }
    if (nav === 'scripts' && this.nav !== 'scripts') {
      this.editorKind = this.browseKind;
      this.editorId =
        (this.current?.node.kind ?? 'flow') === this.browseKind
          ? this.selectedId
          : '';
      this.freshNode = null;
    }
    if (
      nav === 'workbench' &&
      this.browseKind === 'flow' &&
      this.current?.node.flowId !== this.activeFlowId
    ) {
      const steps = this.nodes.filter(
        (r) => r.node.kind !== 'quick' && r.node.flowId === this.activeFlowId,
      );
      this.selectedId =
        steps.find((r) => r.node.isEntry)?.node.id || steps[0]?.node.id || '';
      this.history = [];
    }
    this.nav = nav;
    this.query = '';
    return true;
  }
  select(id: string, reset = false) {
    if (this.nav === 'scripts' && !this.navigate('workbench')) return;
    const node = this.nodes.find((r) => r.node.id === id)?.node;
    const kind = node?.kind ?? 'flow';
    const differentFlow =
      kind !== 'quick' && node?.flowId !== this.activeFlowId;
    if (node?.flowId) this.flowId = node.flowId;
    if (reset || differentFlow || kind === 'quick' || kind !== this.browseKind)
      this.history = [];
    else if (this.selectedId && this.selectedId !== id)
      this.history = [...this.history, this.selectedId];
    this.selectedId = id;
    this.browseKind = kind;
    this.query = '';
    this.nav = 'workbench';
  }
  back() {
    const previous = this.history.at(-1);
    if (previous) {
      this.selectedId = previous;
      const flowId = this.current?.node.flowId;
      if (flowId) this.flowId = flowId;
      this.history = this.history.slice(0, -1);
    }
  }
  edit(record?: NodeRecord) {
    this.editorKind = (record || this.current)?.node.kind ?? 'flow';
    this.editorId = record?.node.id || this.current?.node.id || '';
    const flowId = (record || this.current)?.node.flowId;
    if (flowId) this.flowId = flowId;
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
