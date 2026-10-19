<script lang="ts">
  import {
    ArrowRight,
    ArrowLeft,
    Copy,
    GitBranch,
    ChevronRight,
    Pencil,
    BookOpen,
    FileText,
    FileSpreadsheet,
    Image,
    Film,
    Plus,
    MessageSquare,
    RotateCcw,
  } from 'lucide-svelte';
  import { app } from '../workspace/store.svelte';
  import { api } from '../workspace/api';
  import {
    replyPreview,
    newNode,
    scriptKindName,
    languageName,
    quickCategoryName,
    replyVariables,
  } from '$lib/utils/scripts';
  import type { Material, Snapshot, ScriptKind } from '$lib/types';
  import FlowCanvas from './FlowCanvas.svelte';
  import FlowFullscreen from './FlowFullscreen.svelte';
  import ScriptNavigation from './ScriptNavigation.svelte';
  let node = $derived(app.current?.node);
  let showFlow = $state(false);
  let fullscreenFlow = $state(false);
  let completeFlow = $derived(
    app.nodes.filter(
      (r) => r.node.kind !== 'quick' && r.node.flowId === app.activeFlowId,
    ),
  );
  let preview = $derived(
    node ? replyPreview(node, app.language, app.values) : null,
  );
  let inputs = $derived(node ? replyVariables(node, app.language) : []);
  let flows = $derived(
    app.nodes.filter(
      (n) => n.node.kind !== 'quick' && n.node.flowId === app.activeFlowId,
    ),
  );
  let entries = $derived(flows.filter((n) => n.node.isEntry));
  let quick = $derived(
    app.nodes.filter(
      (n) =>
        n.node.kind === 'quick' &&
        quickCategoryName(n.node) === app.activeQuickCategory,
    ),
  );
  let choices = $derived(
    app.browseKind === 'quick' ? quick : entries.length ? entries : flows,
  );
  function switchFlow(id: string) {
    app.flowId = id;
    const steps = app.nodes.filter(
      (r) => r.node.kind !== 'quick' && r.node.flowId === id,
    );
    const entry = steps.find((r) => r.node.isEntry) || steps[0];
    if (entry) app.select(entry.node.id, true);
    else {
      app.selectedId = '';
      app.history = [];
    }
  }
  function switchCategory(name: string) {
    app.quickCategory = name;
    browse('quick');
  }
  function browse(kind: ScriptKind) {
    app.browseKind = kind;
    const first = kind === 'quick' ? quick[0] : entries[0] || flows[0];
    if (first) app.select(first.node.id, true);
    else {
      app.selectedId = '';
      app.history = [];
    }
  }
  const icon = (kind: string) =>
    ({ image: Image, video: Film, excel: FileSpreadsheet, text: FileText })[
      kind
    ] || FileText;
  function add() {
    app.freshNode =
      app.browseKind === 'quick'
        ? newNode('quick')
        : app.activeFlowId
          ? newNode('flow', app.activeFlowId)
          : null;
    if (app.freshNode?.kind === 'quick')
      app.freshNode.category = app.activeQuickCategory;
    app.editorId = '';
    app.editorKind = app.browseKind;
    app.nav = 'scripts';
  }
</script>

<ScriptNavigation
  kind={app.browseKind}
  onkind={browse}
  onflow={switchFlow}
  oncategory={switchCategory}
/>

{#if !app.nodes.length}
  <div class="empty-workspace">
    <div class="empty-symbol"><MessageSquare size={33} /></div>
    <span class="eyebrow">YOUR FIRST CONVERSATION</span>
    <h2>从一条好话术开始</h2>
    <p class="muted">把常见问题、不同语言的回复和素材组织在一起。</p>
    {#if app.admin}<div class="button-row">
        <button class="button primary" onclick={add}
          ><Plus size={17} />创建第一条话术</button
        ><button
          class="button secondary"
          disabled={!!app.busy}
          onclick={() =>
            app.task(
              '添加示例',
              async () => {
                app.data = await api<Snapshot>('/demo', 'POST');
                app.selectedId = '';
                await app.refresh();
              },
              '已添加示例草稿，请替换为真实内容后发布',
            )}>体验示例流程</button
        >
      </div>
    {:else}<p class="info-note">
        尚无已发布话术，请等待管理员发布或检查同步状态。
      </p>{/if}
  </div>
{:else}
  <div class="workbench-grid">
    {#if showFlow && node && node.kind !== 'quick'}
      <section class="workbench-flow-overview" aria-label="完整流程预览">
        <div class="graph-toolbar">
          <span
            ><GitBranch size={17} />完整流程 · {completeFlow.length} 个步骤</span
          >
          <div class="inline-actions">
            <button class="text-button" onclick={() => (fullscreenFlow = true)}
              >全屏查看流程</button
            ><button class="text-button" onclick={() => (showFlow = false)}
              >收起流程图</button
            >
          </div>
        </div>
        <p class="graph-help">
          高亮为当前话术。点击步骤查看回复，滚轮缩放，拖动空白处平移。
        </p>
        <div class="workbench-flow-canvas">
          {#key app.activeFlowId}<FlowCanvas
              records={completeFlow}
              selectedId={node.id}
              label="销售工作台完整流程图"
              onselect={(id) => app.select(id, true)}
            />{/key}
        </div>
      </section>
    {/if}
    <aside class="flow-list">
      <div class="section-label">
        {app.browseKind === 'quick' ? '快捷问答' : '流程入口'}
        <span>{choices.length}</span>
      </div>
      {#each choices as entry}<button
          class:active={app.selectedId === entry.node.id}
          onclick={() => app.select(entry.node.id, true)}
          ><span class="flow-list-icon"
            >{#if app.browseKind === 'quick'}<MessageSquare
                size={17}
              />{:else}<GitBranch size={17} />{/if}</span
          ><span
            >{entry.node.title}<small
              >{entry.node.kind === 'quick'
                ? entry.node.category || '未分类'
                : app.flows.find((f) => f.id === entry.node.flowId)
                    ?.name}</small
            ></span
          ><ChevronRight size={14} /></button
        >{/each}
      {#if !choices.length}<p class="field-help">
          {app.browseKind === 'quick'
            ? '尚无快捷话术，可在话术与流程中新增。'
            : '尚无流程话术。'}
        </p>{/if}
      <div class="list-note">
        <BookOpen size={17} />
        <p>
          {app.browseKind === 'quick'
            ? '针对客户特定问题，找到对应回复并复制。'
            : '随客户的回答选择分支，合适的资料就在手边。'}
        </p>
      </div>
    </aside>
    <section class="conversation-panel">
      {#if node}
        <div class="conversation-top">
          <span class="eyebrow"
            >{scriptKindName(node.kind)} · {node.kind === 'quick'
              ? node.category || '沟通话术'
              : app.flows.find((f) => f.id === node.flowId)?.name}</span
          >
          <div class="inline-actions">
            {#if node.kind !== 'quick'}<button
                class="text-button"
                onclick={() => (showFlow = !showFlow)}
                ><GitBranch size={16} />{showFlow
                  ? '收起完整流程'
                  : '查看完整流程'}</button
              >{/if}
            <span
              class:personal={app.current?.source === 'personal'}
              class="source-label"
              >{app.current?.source === 'personal'
                ? '个人版本'
                : app.current?.source === 'retained'
                  ? '保留的话术'
                  : app.admin
                    ? '管理员草稿'
                    : '管理员版本'}</span
            ><button
              class="icon-button"
              aria-label="编辑此话术"
              onclick={() => app.edit()}><Pencil size={16} /></button
            >
          </div>
        </div>
        {#if app.history.length}<div class="breadcrumbs">
            <button onclick={() => app.back()}
              ><ArrowLeft size={13} />上一步</button
            >{#each app.history.slice(-3) as id}<span
                >{app.nodes.find((n) => n.node.id === id)?.node.title ||
                  '已移除步骤'}<ChevronRight size={11} /></span
              >{/each}
          </div>{/if}
        <h2 class="conversation-title">{node.title}</h2>
        <p class="question">
          {node.question || '选择合适的回复，与客户开始沟通。'}
        </p>
        {#if inputs.length}<div class="variables">
            <div class="section-label">填写本次回复</div>
            {#each inputs as variable}<label
                ><span
                  >{variable.label || variable.name}{#if variable.required}<span
                      class="required">*</span
                    >{/if}</span
                ><input
                  bind:value={app.values[variable.name]}
                  placeholder={`请输入${variable.label || variable.name}`}
                /></label
              >{/each}
          </div>{/if}
        <div class="reply-card">
          <div class="section-label">
            发送内容 <span>{languageName(app.language)}</span>
          </div>
          {#if preview?.languageMissing}<p class="missing-language">
              尚未维护此语言版本，请选择其他语言或编辑话术。
            </p>{:else}<p class="reply-text">{preview?.text}</p>{/if}
          <div class="reply-actions">
            <span class="field-help"
              >{preview?.missing.length
                ? `待填写：${preview.missing.join('、')}`
                : preview?.languageMissing
                  ? '不会自动替换为其他语言'
                  : '确认内容后复制发送'}</span
            ><button
              class="button primary"
              disabled={!preview?.ready || !!app.busy}
              onclick={() => preview?.ready && app.copy(preview.text)}
              ><Copy size={16} />复制回复</button
            >
          </div>
        </div>
        {#if app.current?.source === 'personal' && app.current.official}<button
            class="text-button"
            onclick={() => app.current && app.reset(app.current)}
            ><RotateCcw size={14} />恢复管理员版本</button
          >{/if}
        {#if node.kind !== 'quick'}<div class="next-step">
            <div class="section-label">
              {node.branches.length ? '客户怎么回答？' : '当前流程已到此处'}
            </div>
            {#if node.branches.length}<div class="branch-buttons">
                {#each node.branches as branch}<button
                    disabled={!app.nodes.some(
                      (n) => n.node.id === branch.targetId,
                    )}
                    onclick={() => app.select(branch.targetId)}
                    ><span>{branch.label}</span><ArrowRight size={17} /></button
                  >{/each}
              </div>{:else}<p class="field-help">
                你可以返回上一步，或搜索其他话术继续沟通。
              </p>{/if}
          </div>{/if}
      {:else}<div class="empty">
          <MessageSquare size={32} />
          <h3>
            {app.browseKind === 'quick' ? '尚无快捷话术' : '尚无流程话术'}
          </h3>
          <p>可在“话术与流程”中添加，或搜索其他话术。</p>
        </div>{/if}
    </section>
    <aside class="related-panel">
      <div class="section-label">
        关联素材 <span>{node?.materialIds.length || 0}</span>
      </div>
      <p class="field-help">选中预览，导出后上传到云机。</p>
      {#each node?.materialIds || [] as id}{@const material =
          app.materials.find((m) => m.id === id)}{#if material}{@const Icon =
            icon(material.kind)}<button
            class="related-card"
            onclick={() => (app.selectedMaterialId = id)}
            ><div class="related-icon"><Icon size={25} /></div>
            <strong>{material.name}</strong><small
              >{material.category || '未分类'}</small
            ><span class="related-action"
              >预览素材 <ArrowRight size={14} /></span
            ></button
          >{:else}<div class="removed-material">
            <FileText size={20} /><strong>素材已移除</strong>
            <p>个人话术已保留，可在编辑页重新关联素材。</p>
          </div>{/if}{/each}{#if !node?.materialIds.length}<div
          class="empty small"
        >
          <BookOpen size={27} />
          <p>
            {node?.kind === 'quick'
              ? '此话术还没有关联素材'
              : '这一步还没有关联素材'}
          </p>
          <button
            class="text-button"
            onclick={() => {
              app.nav = 'library';
            }}>浏览知识库<ArrowRight size={14} /></button
          >
        </div>{/if}
    </aside>
  </div>
{/if}

{#if fullscreenFlow}
  <FlowFullscreen
    records={completeFlow}
    selectedId={node?.id || ''}
    name={app.flows.find((flow) => flow.id === app.activeFlowId)?.name ||
      '完整流程'}
    onclose={() => (fullscreenFlow = false)}
  />
{/if}
