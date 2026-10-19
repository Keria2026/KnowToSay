<script lang="ts">
  import {
    SvelteFlow,
    Background,
    Controls,
    Position,
    MarkerType,
    type Node,
    type Edge,
  } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
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
  } from 'lucide-svelte';
  import { app } from '../workspace/store.svelte';
  import { api } from '../workspace/api';
  import { newNode, languageName, searchScripts } from '$lib/utils/scripts';
  import type { ScriptNode, ScriptKind, Snapshot } from '$lib/types';
  let draft = $state<ScriptNode | null>(null);
  let baseline = $state('');
  let lastId = '';
  let languageCode = $state('');
  let quickQuery = $state('');
  let quickNodes = $derived(app.nodes.filter((r) => r.node.kind === 'quick'));
  let filteredQuick = $derived(searchScripts(quickNodes, quickQuery));
  let flowNodes = $derived(app.nodes.filter((r) => r.node.kind !== 'quick'));
  let current = $derived(app.nodes.find((r) => r.node.id === app.editorId));
  let dirty = $derived(draft && JSON.stringify(draft) !== baseline);
  $effect(() => {
    app.editorDirty = !!dirty;
  });
  let nodes = $state<Node[]>([]);
  let edges = $state<Edge[]>([]);
  $effect(() => {
    const target = app.freshNode || current?.node;
    if (target && target.id !== lastId) {
      draft = JSON.parse(JSON.stringify(target));
      baseline = JSON.stringify(target);
      lastId = target.id;
    }
    if (!target) {
      draft = null;
      lastId = '';
    }
  });
  $effect(() => {
    const items = flowNodes;
    nodes = items.map((r, i) => ({
      id: r.node.id,
      position: r.node.position || {
        x: (i % 3) * 250,
        y: Math.floor(i / 3) * 170,
      },
      data: {
        label: `${r.node.isEntry ? '起点 · ' : ''}${r.node.title}${r.source === 'personal' ? ' · 个人' : ''}`,
      },
      selected: r.node.id === app.editorId,
      sourcePosition: Position.Bottom,
      targetPosition: Position.Top,
    }));
    const ids = new Set(items.map((r) => r.node.id));
    edges = items.flatMap((r) =>
      r.node.branches
        .filter((b) => ids.has(b.targetId))
        .map((b) => ({
          id: b.id,
          source: r.node.id,
          target: b.targetId,
          label: b.label,
          markerEnd: { type: MarkerType.ArrowClosed },
          type: 'smoothstep',
        })),
    );
  });
  function select(id: string) {
    if (dirty && !confirm('当前话术尚未保存，放弃修改并切换？')) return;
    app.freshNode = null;
    app.editorId = id;
  }
  function add() {
    if (dirty && !confirm('当前话术尚未保存，放弃修改？')) return;
    const n = newNode(app.editorKind);
    n.isEntry = app.editorKind === 'flow' && !flowNodes.length;
    app.editorId = '';
    app.freshNode = n;
  }
  function switchKind(kind: ScriptKind) {
    if (app.editorKind === kind) return;
    if (dirty && !confirm('当前话术尚未保存，放弃修改并切换？')) return;
    app.editorDirty = false;
    app.freshNode = null;
    app.editorId = '';
    app.editorKind = kind;
  }
  function changeKind(event: Event) {
    if (!draft) return;
    const input = event.currentTarget as HTMLSelectElement;
    const next = input.value as ScriptKind;
    if (
      next === 'quick' &&
      (draft.branches.length ||
        app.nodes.some((r) =>
          r.node.branches.some((b) => b.targetId === draft!.id),
        ))
    ) {
      input.value = draft.kind ?? 'flow';
      app.error = '请先移除此话术的流程分支和指向它的连接，再改为快捷话术';
      return;
    }
    draft.kind = next;
    if (next === 'quick') {
      draft.isEntry = false;
      draft.position = null;
    }
  }
  async function save() {
    if (!draft) return;
    await app.save(draft);
    if (!app.error) baseline = JSON.stringify(draft);
  }
  async function resetCurrent() {
    if (current && (await app.reset(current))) {
      const node = app.nodes.find((n) => n.node.id === app.editorId)?.node;
      if (node) {
        draft = JSON.parse(JSON.stringify(node));
        baseline = JSON.stringify(node);
      }
    }
  }
  function addLanguage() {
    const code = languageCode.trim();
    if (!draft || !/^[a-z]{2,3}(?:-[A-Za-z0-9]{2,8})*$/.test(code)) {
      app.error = '请输入有效语言代码，例如 ko、en 或 zh-CN';
      return;
    }
    if (!(code in draft.replies)) draft.replies[code] = '';
    languageCode = '';
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
    await app.task('删除话术', async () => {
      app.data = await api<Snapshot>(
        `/scripts/${encodeURIComponent(draft!.id)}`,
        'DELETE',
      );
      app.editorId = '';
      app.freshNode = null;
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
        >{/if}<button class="button primary" onclick={add}
        ><Plus size={17} />{app.editorKind === 'quick'
          ? '新建快捷话术'
          : '新建话术'}</button
      >
    </div>
  </div>
  <div class="script-type-tabs" role="tablist" aria-label="话术管理类型">
    <button
      role="tab"
      aria-selected={app.editorKind === 'flow'}
      class:active={app.editorKind === 'flow'}
      onclick={() => switchKind('flow')}
      ><GitBranch size={17} />流程话术 <span>{flowNodes.length}</span></button
    >
    <button
      role="tab"
      aria-selected={app.editorKind === 'quick'}
      class:active={app.editorKind === 'quick'}
      onclick={() => switchKind('quick')}
      ><MessageSquare size={17} />快捷话术
      <span>{quickNodes.length}</span></button
    >
  </div>
  <p class="field-help">
    {app.editorKind === 'flow'
      ? '按客户回答选择分支，支持汇合与跳转。'
      : '客户特定问题或回复对应的一对一话术，无需建立流程，可直接搜索和复制。'}
  </p>
  <div class="studio-layout">
    {#if app.editorKind === 'flow'}
      <div class="graph-panel">
        <div class="graph-toolbar">
          <span
            ><GitBranch size={16} />{flowNodes.length} 个步骤 · {edges.length} 条连接</span
          ><small>选择步骤，在右侧编辑内容与分支</small>
        </div>
        {#if nodes.length}<div class="graph-canvas">
            <SvelteFlow
              bind:nodes
              bind:edges
              fitView
              nodesDraggable={false}
              nodesConnectable={false}
              elementsSelectable={true}
              deleteKey={null}
              onnodeclick={({ node }) => select(node.id)}
              ><Background gap={22} /><Controls showLock={false} /></SvelteFlow
            >
          </div>{:else}<div class="empty">
            <GitBranch size={38} />
            <h3>建立你的第一条沟通路径</h3>
            <p>新建话术后，通过分支连接到下一步。</p>
            <button class="button primary" onclick={add}>新建话术</button>
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
                  >{record.node.question ||
                    record.node.replies[app.language] ||
                    '暂无此语言回复'}</small
                ><span class="tag"
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
                {quickNodes.length
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
          >{#if dirty}<span class="unsaved">未保存</span>{/if}
        </div>
        <form
          onsubmit={(event) => {
            event.preventDefault();
            void save();
          }}
        >
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
              : '问题 / 内部说明'}<textarea bind:value={draft.question} rows="2"
            ></textarea></label
          >
          <label
            >分类<input
              bind:value={draft.category}
              list="script-categories"
            /><datalist id="script-categories"
              >{#each [...new Set(app.nodes.map((r) => r.node.category))] as category}<option
                  value={category}
                ></option>{/each}</datalist
            ></label
          >
          <label
            >话术类型<select value={draft.kind ?? 'flow'} onchange={changeKind}
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
            <h3>多语言回复</h3>
            {#each Object.keys(draft.replies) as language}<label
                >{languageName(language)}<textarea
                  bind:value={draft.replies[language]}
                  rows="4"
                  placeholder={'输入完整回复，可使用 {变量名}'}
                ></textarea></label
              >{/each}
            <div class="inline-field">
              <input
                bind:value={languageCode}
                placeholder="语言代码，如 en"
                aria-label="新增语言代码"
              /><button
                type="button"
                class="button secondary"
                onclick={addLanguage}>添加</button
              >
            </div>
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
            {#each draft.variables as variable, i}<div class="variable-editor">
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
                      >{#each flowNodes as record}<option value={record.node.id}
                          >{record.node.title}</option
                        >{/each}{#if app.freshNode}<option value={draft.id}
                          >{draft.title}（当前新步骤）</option
                        >{/if}</select
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
                      toggleMaterial(material.id, event.currentTarget.checked)}
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
              disabled={!!app.busy || !draft.title.trim()}
              ><Save size={16} />保存话术</button
            >{#if !app.freshNode && (app.admin || !current?.official)}<button
                type="button"
                class="icon-button danger"
                aria-label="删除话术"
                onclick={remove}><Trash2 size={18} /></button
              >{/if}
          </div>
          <p class="field-help">标识：{draft.id}</p>
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
