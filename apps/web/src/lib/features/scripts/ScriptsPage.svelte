<script lang="ts">
  import { tick } from 'svelte';
  import {
    Plus,
    Save,
    Trash2,
    X,
    GitBranch,
    ArrowRight,
    FileSpreadsheet,
    RotateCcw,
    MessageSquare,
    Search,
    LayoutGrid,
  } from 'lucide-svelte';
  import { app } from '../workspace/store.svelte';
  import { api } from '../workspace/api';
  import {
    newNode,
    languageName,
    searchScripts,
    quickCategoryName,
  } from '$lib/utils/scripts';
  import type {
    ScriptNode,
    ScriptKind,
    Snapshot,
    NodeRecord,
    SaveScripts,
  } from '$lib/types';
  import FlowCanvas from './FlowCanvas.svelte';
  import ScriptNavigation from './ScriptNavigation.svelte';
  import BilingualReplies from './BilingualReplies.svelte';
  import { layoutFlow } from './flow';
  type Point = { x: number; y: number };
  let drafts = $state<Record<string, ScriptNode>>({});
  let quickQuery = $state('');
  let layoutRevision = $state(0);
  let originals = $derived(new Map(app.nodes.map((r) => [r.node.id, r])));
  let records = $derived<NodeRecord[]>([
    ...app.nodes.map((r) => ({ ...r, node: drafts[r.node.id] || r.node })),
    ...Object.values(drafts)
      .filter((n) => !originals.has(n.id))
      .map((node) => ({
        node,
        source: app.admin ? 'draft' : 'personal',
        official: null,
      })),
  ]);
  let changed = $derived(
    Object.values(drafts).filter(
      (n) => JSON.stringify(n) !== JSON.stringify(originals.get(n.id)?.node),
    ),
  );
  let dirty = $derived(changed.length > 0);
  let draft = $derived(drafts[app.editorId] || null);
  let current = $derived(originals.get(app.editorId));
  let quickNodes = $derived(records.filter((r) => r.node.kind === 'quick'));
  let categoryQuickNodes = $derived(
    quickNodes.filter(
      (r) => quickCategoryName(r.node) === app.activeQuickCategory,
    ),
  );
  let filteredQuick = $derived(searchScripts(categoryQuickNodes, quickQuery));
  let allFlowNodes = $derived(records.filter((r) => r.node.kind !== 'quick'));
  let flowNodes = $derived(
    allFlowNodes.filter((r) => r.node.flowId === app.activeFlowId),
  );
  let connectionCount = $derived(
    flowNodes.reduce((sum, r) => sum + r.node.branches.length, 0),
  );
  $effect(() => {
    app.editorDirty = dirty;
  });
  $effect(() => {
    if (app.freshNode) {
      const node = app.freshNode;
      if (node.kind === 'quick') node.category = app.activeQuickCategory;
      if (node.kind !== 'quick' && !node.flowId)
        node.flowId = app.activeFlowId || null;
      drafts[node.id] = JSON.parse(JSON.stringify(node));
      app.editorId = node.id;
      app.freshNode = null;
    }
    if (app.editorId && !drafts[app.editorId] && originals.has(app.editorId)) {
      drafts[app.editorId] = JSON.parse(
        JSON.stringify(originals.get(app.editorId)!.node),
      );
    }
  });
  function editNode(id: string) {
    if (!drafts[id]) {
      const node = records.find((r) => r.node.id === id)?.node;
      if (node) drafts[id] = JSON.parse(JSON.stringify(node));
    }
    return drafts[id];
  }
  function select(id: string) {
    app.editorId = id;
    const node = records.find((r) => r.node.id === id)?.node;
    if (node?.flowId) app.flowId = node.flowId;
    if (node?.kind === 'quick') app.quickCategory = quickCategoryName(node);
  }
  function switchFlow(id: string) {
    app.flowId = id;
    app.editorId = '';
  }
  function switchCategory(name: string) {
    app.quickCategory = name;
    app.editorId = '';
  }
  function add() {
    if (app.editorKind === 'flow' && !app.activeFlowId) return;
    const node = newNode(app.editorKind, app.activeFlowId);
    if (node.kind === 'quick') node.category = app.activeQuickCategory;
    node.isEntry = app.editorKind === 'flow' && !flowNodes.length;
    drafts[node.id] = node;
    app.editorId = node.id;
  }
  function switchKind(kind: ScriptKind) {
    app.editorKind = kind;
    app.browseKind = kind;
    app.editorId = '';
  }
  function changeKind(event: Event) {
    if (!draft) return;
    const input = event.currentTarget as HTMLSelectElement;
    const next = input.value as ScriptKind;
    if (
      next === 'quick' &&
      (draft.branches.length ||
        records.some((r) =>
          r.node.branches.some((b) => b.targetId === draft!.id),
        ))
    ) {
      input.value = draft.kind ?? 'flow';
      app.error = '请先移除此话术的流程分支和指向它的连接，再改为快捷话术';
      return;
    }
    if (next === 'flow' && !app.activeFlowId) {
      input.value = draft.kind || 'flow';
      app.error = '请先新建流程，再转换话术类型';
      return;
    }
    draft.kind = next;
    draft.flowId = next === 'flow' ? app.activeFlowId : null;
    app.editorKind = next;
    if (next === 'quick') {
      draft.category = app.activeQuickCategory;
      draft.isEntry = false;
      draft.position = null;
    }
  }
  async function save() {
    if (!changed.length || app.busy) return;
    const selectedId = app.editorId;
    const kind = draft?.kind || app.editorKind;
    const saved = await app.task(
      '保存话术与流程',
      async () => {
        app.data = await api<Snapshot>('/scripts', 'PUT', {
          nodes: changed,
        } satisfies SaveScripts);
        return true;
      },
      app.admin
        ? '已保存全部修改到主库草稿，发布后同步给业务员'
        : '已保存全部个人修改，管理员更新不会覆盖',
    );
    if (saved) {
      drafts = {};
      app.selectedId = selectedId;
      app.browseKind = kind;
      app.editorKind = kind;
    }
  }
  async function resetCurrent() {
    if (current && (await app.reset(current))) delete drafts[app.editorId];
  }
  function moveNodes(positions: { id: string; position: Point }[]) {
    for (const { id, position } of positions) {
      const node = editNode(id);
      if (node) node.position = { ...position };
    }
  }
  function arrange() {
    moveNodes(
      [...layoutFlow(flowNodes)].map(([id, position]) => ({ id, position })),
    );
    layoutRevision++;
  }
  let branchDialog: HTMLDialogElement;
  let pendingBranch = $state<{
    source: string;
    target: string;
    branchId: string;
    label: string;
    title: string;
    position: Point | null;
  } | null>(null);
  async function connect(
    source: string,
    target: string,
    position: Point | null = null,
    branchId = '',
  ) {
    const existing = records
      .find((r) => r.node.id === source)
      ?.node.branches.find((b) => b.id === branchId);
    pendingBranch = {
      source,
      target,
      branchId,
      position,
      title: '',
      label: existing?.label || '',
    };
    await tick();
    branchDialog.showModal();
  }
  function commitBranch() {
    if (!pendingBranch || !pendingBranch.label.trim()) return;
    const source = editNode(pendingBranch.source);
    if (!source) return;
    let target = pendingBranch.target;
    if (pendingBranch.position) {
      const node = newNode('flow', source.flowId ?? null);
      node.title = pendingBranch.title.trim();
      if (!node.title) return;
      node.position = pendingBranch.position;
      drafts[node.id] = node;
      target = node.id;
    }
    const branch = source.branches.find(
      (b) => b.id === pendingBranch!.branchId,
    );
    if (branch) branch.label = pendingBranch.label.trim();
    else
      source.branches.push({
        id: crypto.randomUUID(),
        label: pendingBranch.label.trim(),
        targetId: target,
      });
    app.editorId = pendingBranch.position ? target : source.id;
    branchDialog.close();
  }
  function removeBranch() {
    if (!pendingBranch) return;
    const node = editNode(pendingBranch.source);
    if (node)
      node.branches = node.branches.filter(
        (b) => b.id !== pendingBranch!.branchId,
      );
    branchDialog.close();
  }
  function toggleMaterial(id: string, selected: boolean) {
    if (draft)
      draft.materialIds = selected
        ? [...draft.materialIds, id]
        : draft.materialIds.filter((v) => v !== id);
  }
  async function remove() {
    if (!draft || !confirm('删除这条话术？请检查其他话术是否仍引用它。'))
      return;
    const id = draft.id;
    if (!originals.has(id)) {
      delete drafts[id];
      app.editorId = '';
      return;
    }
    await app.task('删除话术', async () => {
      app.data = await api<Snapshot>(
        `/scripts/${encodeURIComponent(id)}`,
        'DELETE',
      );
      delete drafts[id];
      app.editorId = '';
    });
  }
</script>

<section class="scripts-page">
  <div class="page-heading">
    <div>
      <span class="eyebrow">SCRIPT STUDIO</span>
      <h2>话术与流程</h2>
      <p class="muted">
        {app.admin
          ? '编辑主库草稿，整理好后统一发布。'
          : '你的个人分支独立保存，管理员更新不会覆盖修改。'}
      </p>
    </div>
    <div class="button-row">
      {#if app.admin}<button
          class="button secondary"
          onclick={() => app.navigate('library')}
          ><FileSpreadsheet size={16} />从 Excel 导入</button
        >{/if}<button
        class="button primary"
        disabled={!!app.busy ||
          (app.editorKind === 'flow' && !app.activeFlowId)}
        onclick={add}
        ><Plus size={17} />{app.editorKind === 'quick'
          ? '新建快捷话术'
          : '新建话术'}</button
      >
    </div>
  </div>
  <ScriptNavigation
    kind={app.editorKind}
    onkind={switchKind}
    onflow={switchFlow}
    oncategory={switchCategory}
    editable
    flowCount={allFlowNodes.length}
    quickCount={quickNodes.length}
  />
  <div class="studio-layout">
    {#if app.editorKind === 'flow'}
      <div class="graph-panel">
        <div class="graph-toolbar">
          <span
            ><GitBranch size={16} />{flowNodes.length} 个步骤 · {connectionCount}
            条连接</span
          >
          <div class="button-row">
            <button class="text-button" disabled={!!app.busy} onclick={arrange}
              ><LayoutGrid size={16} />自动排列</button
            >
            <button
              class="button primary"
              disabled={!dirty || !!app.busy}
              onclick={save}><Save size={16} />保存流程</button
            >
          </div>
        </div>
        <p class="graph-help">
          拖动节点排版；从右侧圆点拉箭头到已有节点或空白处新增分支。点击连线可修改条件。
        </p>
        {#if flowNodes.length}<div
            class="graph-canvas"
            role="tabpanel"
            aria-label={app.flows.find((f) => f.id === app.activeFlowId)?.name}
          >
            {#key `${app.activeFlowId}:${layoutRevision}`}
              <FlowCanvas
                records={flowNodes}
                selectedId={app.editorId}
                editable={!app.busy}
                onselect={select}
                onpositions={moveNodes}
                onconnect={(source, target) => connect(source, target)}
                onblankconnect={(source, position) =>
                  connect(source, '', position)}
                onedge={(source, id) => {
                  const branch = flowNodes
                    .find((r) => r.node.id === source)
                    ?.node.branches.find((b) => b.id === id);
                  if (branch) void connect(source, branch.targetId, null, id);
                }}
              />
            {/key}
          </div>{:else}<div class="empty">
            <GitBranch size={38} />
            <h3>
              {app.activeFlowId ? '这个流程还没有话术' : '先新建一条流程'}
            </h3>
            <p>
              {app.activeFlowId
                ? '新建话术后，拉出箭头连接到下一步。'
                : '点击上方“新建流程”，为不同沟通场景建立独立页面。'}
            </p>
            {#if app.activeFlowId}<button class="button primary" onclick={add}
                >新建话术</button
              >{/if}
          </div>{/if}
      </div>
    {:else}
      <div class="script-library-panel">
        <label class="script-list-search"
          ><Search size={18} /><input
            type="search"
            bind:value={quickQuery}
            aria-label="筛选快捷话术"
            placeholder="搜索客户问题、题目或回复…"
          /></label
        >
        <div class="script-library-list">
          {#each filteredQuick as record}<button
              class="search-result"
              class:selected={app.editorId === record.node.id}
              onclick={() => select(record.node.id)}
            >
              <MessageSquare size={19} /><span
                ><strong>{record.node.title}</strong><small
                  >{record.node.question}</small
                ><BilingualReplies node={record.node} /><span class="tag"
                  >{record.node.category || '未分类'}{record.source ===
                  'personal'
                    ? ' · 个人版'
                    : ''}</span
                ></span
              >
            </button>{/each}
          {#if !filteredQuick.length}<div class="empty">
              <MessageSquare size={32} />
              <h3>
                {categoryQuickNodes.length
                  ? '没有匹配的快捷话术'
                  : '添加第一条快捷话术'}
              </h3>
              <p>例如：客户说“太贵了”，对应一段准备好的回复。</p>
            </div>{/if}
        </div>
      </div>
    {/if}
    <aside class="node-editor">
      {#if draft}<div class="editor-heading">
          <span class="source-label personal"
            >{app.admin ? '保存到主库草稿' : '保存到个人分支'}</span
          >{#if dirty}<span class="unsaved">{changed.length} 条未保存</span
            >{/if}
        </div>
        <form
          onsubmit={(event) => {
            event.preventDefault();
            void save();
          }}
        >
          <fieldset disabled={!!app.busy}>
            <label
              >题目<input
                bind:value={draft.title}
                required
                maxlength="160"
              /></label
            >
            <label
              >{draft.kind === 'quick'
                ? '客户问题 / 特定回复'
                : '问题 / 内部说明'}<textarea
                bind:value={draft.question}
                rows="2"
              ></textarea></label
            >
            {#if draft.kind === 'quick'}
              <label
                >分类<select
                  aria-label="分类"
                  value={quickCategoryName(draft)}
                  onchange={(event) => {
                    if (draft) {
                      draft.category = event.currentTarget.value;
                      app.quickCategory = draft.category;
                    }
                  }}
                >
                  {#each app.quickGroups as category}<option value={category.id}
                      >{category.name}</option
                    >{/each}
                </select></label
              >
            {/if}
            <label
              >话术类型<select
                value={draft.kind ?? 'flow'}
                onchange={changeKind}
                ><option value="flow">流程话术</option><option value="quick"
                  >快捷话术</option
                ></select
              ></label
            >
            {#if draft.kind !== 'quick'}
              <label class="check-label"
                ><input
                  type="checkbox"
                  bind:checked={draft.isEntry}
                />作为流程入口</label
              >
            {/if}
            <div class="editor-section">
              <h3>话术内容</h3>
              {#each ['zh-CN', 'ko', ...Object.keys(draft.replies).filter((key) => !['zh-CN', 'ko'].includes(key))] as language}<label
                  >{languageName(language)}<textarea
                    bind:value={draft.replies[language]}
                    rows="4"
                    placeholder={'输入完整回复，可使用 {变量名}'}
                  ></textarea></label
                >{/each}
            </div>
            <div class="editor-section">
              <div class="section-title">
                <h3>变量</h3>
                <button
                  type="button"
                  class="text-button"
                  onclick={() =>
                    draft?.variables.push({
                      name: '',
                      label: '',
                      required: true,
                    })}><Plus size={14} />添加</button
                >
              </div>
              {#each draft.variables as variable, i}<div
                  class="variable-editor"
                >
                  <input
                    aria-label="变量名"
                    bind:value={variable.name}
                    placeholder="变量名"
                  /><input
                    aria-label="变量显示名称"
                    bind:value={variable.label}
                    placeholder="显示名称"
                  /><label class="check-label"
                    ><input
                      type="checkbox"
                      bind:checked={variable.required}
                    />必填</label
                  ><button
                    type="button"
                    class="icon-button"
                    aria-label="删除变量"
                    onclick={() => draft?.variables.splice(i, 1)}
                    ><X size={14} /></button
                  >
                </div>{/each}
              <p class="field-help">
                回复中使用花括号引用变量，例如：您好，{'{客户姓名}'}。
              </p>
            </div>
            {#if draft.kind !== 'quick'}<div class="editor-section">
                <div class="section-title">
                  <h3>下一步分支</h3>
                  <button
                    type="button"
                    class="text-button"
                    onclick={() =>
                      draft?.branches.push({
                        id: crypto.randomUUID(),
                        label: '',
                        targetId: '',
                      })}><Plus size={14} />添加</button
                  >
                </div>
                {#each draft.branches as branch, i}<div class="branch-editor">
                    <input
                      aria-label="客户回答"
                      bind:value={branch.label}
                      placeholder="客户回答，如：吃了"
                      required
                    />
                    <div class="inline-field">
                      <ArrowRight size={14} /><select
                        bind:value={branch.targetId}
                        aria-label="跳转到的话术"
                        required
                        ><option value="">选择下一步</option
                        >{#each flowNodes as record}<option
                            value={record.node.id}>{record.node.title}</option
                          >{/each}</select
                      ><button
                        type="button"
                        class="icon-button"
                        aria-label="删除分支"
                        onclick={() => draft?.branches.splice(i, 1)}
                        ><X size={15} /></button
                      >
                    </div>
                  </div>{/each}
              </div>{/if}
            <div class="editor-section">
              <h3>关联素材</h3>
              {#each draft.materialIds.filter((id) => !app.materials.some((m) => m.id === id)) as id}<div
                  class="removed-material"
                >
                  素材已移除<button
                    type="button"
                    class="text-button"
                    onclick={() => toggleMaterial(id, false)}>取消关联</button
                  >
                </div>{/each}
              <div class="material-options">
                {#each app.materials as material}<label class="check-label"
                    ><input
                      type="checkbox"
                      checked={draft.materialIds.includes(material.id)}
                      onchange={(event) =>
                        toggleMaterial(
                          material.id,
                          event.currentTarget.checked,
                        )}
                    />{material.name}</label
                  >{/each}{#if !app.materials.length}<p class="field-help">
                    知识库尚无素材。
                  </p>{/if}
              </div>
            </div>
            {#if current?.official && current.source === 'personal'}<details>
                <summary>查看当前管理员版本</summary
                >{#each Object.entries(current.official.replies) as [language, text]}<h4
                  >
                    {languageName(language)}
                  </h4>
                  <p class="text-preview">{text}</p>{/each}<button
                  type="button"
                  class="text-button"
                  onclick={resetCurrent}
                  ><RotateCcw size={14} />放弃个人修改，恢复管理员版本</button
                >
              </details>{/if}
            <div class="editor-footer">
              <button
                class="button primary"
                disabled={!!app.busy || !dirty || !draft.title.trim()}
                ><Save size={16} />保存话术</button
              >{#if app.admin || !current?.official}<button
                  type="button"
                  class="icon-button danger"
                  aria-label="删除话术"
                  onclick={remove}><Trash2 size={18} /></button
                >{/if}
            </div>
            <p class="field-help">
              保存会包含所有未保存的话术、布局与连接。标识：{draft.id}
            </p>
          </fieldset>
        </form>
      {:else}<div class="empty small">
          <GitBranch size={32} />
          <h3>
            {app.editorKind === 'quick' ? '选择一条快捷话术' : '选择一个步骤'}
          </h3>
          <p>
            {app.editorKind === 'quick'
              ? '选择客户问题，维护对应的多语言回复。'
              : '在画布上点击话术，编辑回复、变量和下一步。'}
          </p>
        </div>{/if}
    </aside>
  </div>
</section>

<dialog
  bind:this={branchDialog}
  class="dialog branch-dialog"
  aria-labelledby="branch-dialog-title"
  onclose={() => (pendingBranch = null)}
>
  {#if pendingBranch}
    <form
      onsubmit={(event) => {
        event.preventDefault();
        commitBranch();
      }}
    >
      <div class="dialog-header">
        <h2 id="branch-dialog-title">
          {pendingBranch.branchId ? '修改分支' : '新增分支'}
        </h2>
        <button
          type="button"
          class="icon-button"
          aria-label="关闭分支编辑"
          onclick={() => branchDialog.close()}><X size={18} /></button
        >
      </div>
      <p class="field-help">
        从「{records.find((r) => r.node.id === pendingBranch!.source)?.node
          .title}」{pendingBranch.position
          ? '创建子话术'
          : `连接到「${records.find((r) => r.node.id === pendingBranch!.target)?.node.title}」`}
      </p>
      <label
        >分支条件 / 客户回答<input
          bind:value={pendingBranch.label}
          required
          maxlength="200"
          placeholder="例如：吃了、没吃、想了解价格"
        /></label
      >
      {#if pendingBranch.position}<label
          >子话术题目<input
            bind:value={pendingBranch.title}
            required
            maxlength="160"
            placeholder="输入下一步话术题目"
          /></label
        >{/if}
      <p class="field-help">确认后可在右侧维护回复；点击“保存流程”统一保存。</p>
      <div class="dialog-actions">
        {#if pendingBranch.branchId}<button
            type="button"
            class="button secondary"
            onclick={removeBranch}><Trash2 size={16} />删除分支</button
          >{/if}
        <button
          type="button"
          class="button secondary"
          onclick={() => branchDialog.close()}>取消</button
        >
        <button class="button primary">确认分支</button>
      </div>
    </form>
  {/if}
</dialog>
