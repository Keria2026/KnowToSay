<script lang="ts">
  import { onMount } from 'svelte';
  import { Search, X, Expand, Copy, ArrowLeft, BookOpen } from 'lucide-svelte';
  import { app } from '../workspace/store.svelte';
  import {
    languageName,
    replyPreview,
    replyVariables,
    quickCategoryName,
  } from '$lib/utils/scripts';
  import { folderPath, folderOf, libraryItems } from '../knowledge/folders';
  import SearchScopePicker from '../workspace/SearchScopePicker.svelte';
  import SearchResults from '../workspace/SearchResults.svelte';
  import ScriptResult from './ScriptResult.svelte';
  import MaterialResult from '../knowledge/MaterialResult.svelte';
  import type { ScriptKind } from '$lib/types';
  let section = $state<ScriptKind | 'library'>(app.browseKind);
  let query = $state('');
  let category = $state('');
  let materialKind = $state('all');
  let limit = $state(50);
  let flow = $derived(
    app.nodes.filter(
      (r) => r.node.kind !== 'quick' && r.node.flowId === app.activeFlowId,
    ),
  );
  let quick = $derived(
    app.nodes.filter(
      (r) =>
        r.node.kind === 'quick' &&
        quickCategoryName(r.node) === app.activeQuickCategory,
    ),
  );
  let node = $derived(app.current?.node);
  let variables = $derived(node ? replyVariables(node, app.language) : []);
  let preview = $derived(
    node ? replyPreview(node, app.language, app.values) : null,
  );
  let materials = $derived(
    libraryItems(app.materials).filter(
      (m) =>
        (materialKind === 'all' || m.kind === materialKind) &&
        (!category || folderOf(m, app.folders) === category),
    ),
  );
  let categories = $derived(
    app.folders.map((f) => ({ id: f.id, name: folderPath(app.folders, f.id) })),
  );
  onMount(() => {
    if (section === 'flow' && node?.flowId !== app.activeFlowId)
      chooseGroup(app.activeFlowId);
  });
  function chooseGroup(id: string) {
    if (section === 'flow') {
      app.flowId = id;
      const records = app.nodes.filter(
        (r) => r.node.kind !== 'quick' && r.node.flowId === id,
      );
      const entry = records.find((r) => r.node.isEntry) || records[0];
      if (entry) app.focusScript(entry.node.id, true);
      else {
        app.selectedId = '';
        app.history = [];
      }
    } else {
      app.quickCategory = id;
      const record = app.nodes.find(
        (r) => r.node.kind === 'quick' && quickCategoryName(r.node) === id,
      );
      if (record) app.focusScript(record.node.id, true);
      else {
        app.selectedId = '';
        app.history = [];
      }
    }
  }
  function browse(next: ScriptKind | 'library') {
    section = next;
    query = '';
    limit = 50;
    if (next !== 'library') {
      app.browseKind = next;
      if (
        next !== 'flow' ||
        node?.kind === 'quick' ||
        node?.flowId !== app.activeFlowId
      )
        chooseGroup(
          next === 'flow' ? app.activeFlowId : app.activeQuickCategory,
        );
    }
  }
</script>

<div class="compact-tool" aria-label="置顶销售工具">
  <header class="compact-toolbar">
    <strong data-tauri-drag-region>KnowToSay</strong><label
      class="compact-language"
      ><span class="visually-hidden">发送语言</span><select
        bind:value={app.language}
        aria-label="发送语言"
        >{#each app.languages as language}<option value={language}
            >{languageName(language)}</option
          >{/each}</select
      ></label
    ><button
      class="icon-button"
      aria-label="切换完整窗口"
      title="切回完整窗口进行编辑和管理"
      onclick={() => app.toggleCompact()}
      disabled={!!app.busy}><Expand size={18} /></button
    >
  </header>
  <div class="compact-search">
    <Search size={18} /><input
      type="search"
      bind:value={query}
      aria-label="全局搜索"
      placeholder="搜索并复制…"
      onkeydown={(e) => {
        if (e.key === 'Escape') {
          query = '';
          e.stopPropagation();
        }
      }}
    />{#if query}<button
        class="icon-button"
        aria-label="清空搜索"
        onclick={() => (query = '')}><X size={16} /></button
      >{/if}
  </div>
  <SearchScopePicker bind:scope={app.searchScope} />
  {#if query.trim()}<div class="compact-content">
      <SearchResults {query} dense onclear={() => (query = '')} />
    </div>
  {:else}
    <div class="compact-use-tabs" role="tablist" aria-label="悬浮窗内容">
      {#each [['flow', '流程'], ['quick', '快捷话术'], ['library', '知识库']] as [id, label]}<button
          role="tab"
          class:active={section === id}
          aria-selected={section === id}
          onclick={() => browse(id as ScriptKind | 'library')}>{label}</button
        >{/each}
    </div>
    <div class="compact-content">
      {#if section !== 'library'}<label class="compact-group"
          ><span class="visually-hidden"
            >{section === 'flow' ? '当前流程' : '快捷分类'}</span
          ><select
            aria-label={section === 'flow' ? '当前流程' : '快捷分类'}
            value={section === 'flow'
              ? app.activeFlowId
              : app.activeQuickCategory}
            onchange={(e) => chooseGroup(e.currentTarget.value)}
            >{#each section === 'flow' ? app.flows : app.quickGroups as group}<option
                value={group.id}>{group.name}</option
              >{/each}</select
          ></label
        >{/if}
      {#if section === 'flow'}
        {#if node && node.kind !== 'quick' && node.flowId === app.activeFlowId}<section
            class="compact-conversation"
          >
            <div class="section-title">
              <h2>{node.title}</h2>
              <button
                class="text-button"
                onclick={() => (app.replyId = node.id)}>详情</button
              >
            </div>
            {#if app.history.length}<button
                class="text-button"
                onclick={() => app.back()}><ArrowLeft size={15} />上一步</button
              >{/if}
            {#if node.replies['zh-CN']}<p class="compact-reference">
                {node.replies['zh-CN']}
              </p>{:else if node.question}<p class="compact-reference">
                {node.question}
              </p>{/if}
            {#each variables as variable}<label
                >{variable.label || variable.name}{variable.required
                  ? ' *'
                  : ''}<input
                  bind:value={app.values[variable.name]}
                  placeholder={`请输入${variable.label || variable.name}`}
                /></label
              >{/each}
            <div class="compact-reply">
              <span class="section-label"
                >复制内容 · {languageName(app.language)}</span
              >
              <p>
                {preview?.languageMissing
                  ? '尚未维护此语言版本'
                  : preview?.text}
              </p>
            </div>
            <div class="compact-copy">
              <span class="field-help"
                >{preview?.missing.length
                  ? `待填写：${preview.missing.join('、')}`
                  : '复制后粘贴到聊天窗口'}</span
              ><button
                class="button primary"
                disabled={!preview?.ready || !!app.busy}
                onclick={() => preview?.ready && app.copy(preview.text)}
                ><Copy size={16} />复制回复</button
              >
            </div>
            {#if node.branches.length}<div class="compact-branches">
                <span class="section-label">客户怎么回答？</span
                >{#each node.branches as branch}<button
                    class="button secondary"
                    disabled={!app.nodes.some(
                      (r) => r.node.id === branch.targetId,
                    )}
                    onclick={() => app.focusScript(branch.targetId)}
                    >{branch.label}</button
                  >{/each}
              </div>{/if}
            {#if node.materialIds.length}<details class="compact-linked">
                <summary
                  ><BookOpen size={16} />关联素材 · {node.materialIds
                    .length}</summary
                >{#each node.materialIds as id}{@const material =
                    app.materials.find(
                      (m) => m.id === id,
                    )}{#if material}<MaterialResult
                      hit={{ material, rows: [] }}
                    />{:else}<p class="field-help">素材已移除</p>{/if}{/each}
              </details>{/if}
          </section>{:else}<div class="empty small">
            <p>{flow.length ? '请选择流程中的话术' : '此流程还没有可用话术'}</p>
          </div>{/if}
      {:else if section === 'quick'}<div class="compact-result-list">
          {#each quick.slice(0, limit) as record (record.node.id)}<ScriptResult
              {record}
            />{/each}{#if !quick.length}<p class="field-help">
              此分类还没有快捷话术。
            </p>{/if}{#if quick.length > limit}<button
              class="text-button"
              onclick={() => (limit += 50)}>显示更多话术</button
            >{/if}
        </div>
      {:else}<div class="compact-library-filters">
          <select aria-label="素材类型" bind:value={materialKind}
            ><option value="all">全部素材</option><option value="case"
              >案例</option
            ><option value="text">文字</option><option value="image"
              >图片</option
            ><option value="video">视频</option><option value="excel"
              >表格</option
            ></select
          ><select aria-label="素材文件夹" bind:value={category}
            ><option value="">全部文件夹</option
            >{#each categories as folder}<option value={folder.id}
                >{folder.name}</option
              >{/each}</select
          >
        </div>
        <div class="compact-result-list">
          {#each materials.slice(0, limit) as material (material.id)}<MaterialResult
              hit={{ material, rows: [] }}
            />{/each}{#if !materials.length}<p class="field-help">
              没有匹配的素材。
            </p>{/if}{#if materials.length > limit}<button
              class="text-button"
              onclick={() => (limit += 50)}>显示更多素材</button
            >{/if}
        </div>{/if}
    </div>
  {/if}
  <footer class="compact-status">
    <span class="status-dot"></span>本地可用 · v{app.workspace?.version ||
      0}<span>{app.current?.source === 'personal' ? '个人话术' : ''}</span>
  </footer>
</div>
