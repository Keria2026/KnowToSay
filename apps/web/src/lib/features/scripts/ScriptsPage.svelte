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
  } from 'lucide-svelte';
  import { app } from '../workspace/store.svelte';
  import { api } from '../workspace/api';
  import { newNode, languageName } from '$lib/utils/scripts';
  import type { ScriptNode, Snapshot } from '$lib/types';
  let draft = $state<ScriptNode | null>(null);
  let baseline = $state('');
  let lastId = '';
  let languageCode = $state('');
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
    const items = app.nodes;
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
    const n = newNode();
    n.isEntry = !app.nodes.length;
    app.editorId = '';
    app.freshNode = n;
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
        ><Plus size={17} />新建话术</button
      >
    </div>
  </div>
  <div class="studio-layout">
    <div class="graph-panel">
      <div class="graph-toolbar">
        <span
          ><GitBranch size={16} />{app.nodes.length} 个步骤 · {edges.length} 条连接</span
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
            >问题 / 内部说明<textarea bind:value={draft.question} rows="2"
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
          <label class="check-label"
            ><input
              type="checkbox"
              bind:checked={draft.isEntry}
            />作为流程入口</label
          >
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
          <div class="editor-section">
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
                    >{#each app.nodes as record}<option value={record.node.id}
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
          </div>
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
          <h3>选择一个步骤</h3>
          <p>在画布上点击话术，编辑回复、变量和下一步。</p>
        </div>{/if}
    </aside>
  </div>
</section>
