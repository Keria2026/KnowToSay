import { invoke } from '@tauri-apps/api/core';
import { api, native, copyText } from './api';
import { globalScope } from './search';
import type { SearchScope } from './search';
import { quickCategoryName } from '$lib/utils/scripts';
import {
  groupOrderKey,
  readGroupOrder,
  orderedGroupIds,
  orderGroups,
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
  searchScope = $state<SearchScope>(globalScope());
  replyId = $state('');
  materialQuery = $state<string | null>(null);
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
  flows = $derived(orderGroups(this.data?.flows ?? [], this.flowOrder));
  activeFlowId = $derived(
    this.flows.some((f) => f.id === this.flowId)
      ? this.flowId
      : this.flows[0]?.id || '',
  );
  nodes = $derived(this.data?.nodes ?? []);
  quickCategory = $state('');
  quickCategoryOrder = $state<string[]>([]);
  quickCategoryItems = $derived(
    [
      ...new Set([
        '未分类',
        ...(this.data?.quickCategories ?? []),
        ...this.nodes
          .filter((r) => r.node.kind === 'quick')
          .map((r) => quickCategoryName(r.node)),
      ]),
    ].map((name) => ({ id: name, name })),
  );
  quickGroups = $derived(
    orderGroups(this.quickCategoryItems, this.quickCategoryOrder),
  );
  occupiedQuickCategories = $derived(
    new Set(
      this.nodes
        .filter((r) => r.node.kind === 'quick')
        .map((r) => quickCategoryName(r.node)),
    ),
  );
  activeQuickCategory = $derived(
    this.quickGroups.some((g) => g.id === this.quickCategory)
      ? this.quickCategory
      : this.quickGroups.find((g) => this.occupiedQuickCategories.has(g.id))
          ?.id ||
          this.quickGroups[0]?.id ||
          '未分类',
  );
  editorKind = $state<ScriptKind>('flow');
  browseKind = $state<ScriptKind>('flow');
  editorDirty = $state(false);
  freshNode = $state<ScriptNode | null>(null);
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

  restoreGroupOrders() {
    if (!this.workspace) {
      this.flowOrder = [];
      this.quickCategoryOrder = [];
      return;
    }
    try {
      for (const kind of ['flow', 'quick'] as const) {
        const items =
          kind === 'flow' ? (this.data?.flows ?? []) : this.quickCategoryItems;
        const key = groupOrderKey(this.workspace.groupId, kind);
        const order = orderedGroupIds(
          items,
          readGroupOrder(localStorage.getItem(key)),
        );
        localStorage.setItem(key, JSON.stringify(order));
        if (kind === 'flow') this.flowOrder = order;
        else this.quickCategoryOrder = order;
      }
    } catch {
      this.error = '无法读取或保存本机标签顺序，请检查本机存储后重试';
    }
  }
  saveGroupOrder(kind: ScriptKind, ids: string[]) {
    if (!this.workspace) return false;
    this.error = '';
    try {
      const items =
        kind === 'flow' ? (this.data?.flows ?? []) : this.quickCategoryItems;
      const order = orderedGroupIds(items, ids);
      const selected =
        kind === 'flow' ? this.activeFlowId : this.activeQuickCategory;
      localStorage.setItem(
        groupOrderKey(this.workspace.groupId, kind),
        JSON.stringify(order),
      );
      if (kind === 'flow') {
        this.flowId = selected;
        this.flowOrder = order;
      } else {
        this.quickCategory = selected;
        this.quickCategoryOrder = order;
      }
      return true;
    } catch {
      this.error = '标签顺序保存失败，请检查本机存储后重试';
      return false;
    }
  }
  saveFlowOrder(ids: string[]) {
    return this.saveGroupOrder('flow', ids);
  }
  saveQuickCategoryOrder(ids: string[]) {
    return this.saveGroupOrder('quick', ids);
  }
  async refresh() {
    const data = await api<Snapshot>('/snapshot');
    this.data = data;
    if (
      !this.selectedId ||
      !data.nodes.some((n) => n.node.id === this.selectedId)
    ) {
      const candidates =
        this.browseKind === 'quick' && this.quickCategory
          ? data.nodes.filter(
              (r) =>
                r.node.kind === 'quick' &&
                quickCategoryName(r.node) === this.activeQuickCategory,
            )
          : this.browseKind === 'flow' &&
              this.flowId &&
              data.flows.some((f) => f.id === this.flowId)
            ? data.nodes.filter(
                (r) => r.node.kind !== 'quick' && r.node.flowId === this.flowId,
              )
            : data.nodes;
      this.selectedId =
        candidates.find((r) => r.node.isEntry)?.node.id ||
        candidates[0]?.node.id ||
        '';
    }
    if (this.current) this.browseKind = this.current.node.kind ?? 'flow';
    const selectedFlowId = this.current?.node.flowId;
    if (selectedFlowId) this.flowId = selectedFlowId;
    if (this.current?.node.kind === 'quick')
      this.quickCategory = quickCategoryName(this.current.node);
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
    if (
      nav === 'workbench' &&
      this.browseKind === 'quick' &&
      (this.current?.node.kind !== 'quick' ||
        quickCategoryName(this.current.node) !== this.activeQuickCategory)
    ) {
      this.selectedId =
        this.nodes.find(
          (r) =>
            r.node.kind === 'quick' &&
            quickCategoryName(r.node) === this.activeQuickCategory,
        )?.node.id || '';
      this.history = [];
    }
    this.nav = nav;
    this.query = '';
    return true;
  }
  select(id: string, reset = false) {
    if (this.nav === 'scripts' && !this.navigate('workbench')) return;
    this.focusScript(id, reset);
    this.query = '';
    this.nav = 'workbench';
  }
  focusScript(id: string, reset = false) {
    const node = this.nodes.find((r) => r.node.id === id)?.node;
    const kind = node?.kind ?? 'flow';
    const differentFlow =
      kind !== 'quick' && node?.flowId !== this.activeFlowId;
    if (node?.flowId) this.flowId = node.flowId;
    if (node?.kind === 'quick') this.quickCategory = quickCategoryName(node);
    if (reset || differentFlow || kind === 'quick' || kind !== this.browseKind)
      this.history = [];
    else if (this.selectedId && this.selectedId !== id)
      this.history = [...this.history, this.selectedId];
    this.selectedId = id;
    this.browseKind = kind;
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
    const editedNode = (record || this.current)?.node;
    if (editedNode?.kind === 'quick')
      this.quickCategory = quickCategoryName(editedNode);
    const flowId = editedNode?.flowId;
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
          const restored = this.nodes.find(
            (r) => r.node.id === record.node.id,
          )?.node;
          if (restored?.kind === 'quick')
            this.quickCategory = quickCategoryName(restored);
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
