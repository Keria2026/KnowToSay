<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import {
    Search,
    MessageSquare,
    GitBranch,
    BookOpen,
    Radio,
    Settings,
    PanelRightClose,
    Expand,
    ChevronDown,
    X,
    Check,
    RefreshCw,
    ArrowUpRight,
    LoaderCircle,
  } from 'lucide-svelte';
  import { app } from '$lib/features/workspace/store.svelte';
  import { api, native } from '$lib/features/workspace/api';
  import { languageName } from '$lib/utils/scripts';
  import type { SyncStatus } from '$lib/types';
  import Onboarding from '$lib/features/workspace/Onboarding.svelte';
  import Workbench from '$lib/features/scripts/Workbench.svelte';
  import ScriptsPage from '$lib/features/scripts/ScriptsPage.svelte';
  import LibraryPage from '$lib/features/knowledge/LibraryPage.svelte';
  import MaterialDialog from '$lib/features/knowledge/MaterialDialog.svelte';
  import SearchResults from '$lib/features/workspace/SearchResults.svelte';
  import SyncPage from '$lib/features/workspace/SyncPage.svelte';
  import SettingsPage from '$lib/features/workspace/SettingsPage.svelte';
  import QuickSearch from '$lib/features/workspace/QuickSearch.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  let quickSearchMode = $state(false);
  let searchInput = $state<HTMLInputElement>();
  const navigation = [
    { id: 'workbench', name: '销售工作台', icon: MessageSquare },
    { id: 'scripts', name: '话术与流程', icon: GitBranch },
    { id: 'library', name: '知识库', icon: BookOpen },
    { id: 'sync', name: '发布与同步', icon: Radio },
    { id: 'settings', name: '工作组与备份', icon: Settings },
  ];
  const titles: Record<string, string> = {
    workbench: '让每次沟通，都有准备。',
    scripts: '话术与流程',
    library: '知识库',
    sync: '发布与同步',
    settings: '工作组与备份',
  };
  $effect(() => {
    const group = app.workspace?.groupId;
    const flows = app.data?.flows;
    const categories = app.data?.quickCategories;
    if (group && flows && categories) untrack(() => app.restoreGroupOrders());
  });
  onMount(() => {
    quickSearchMode = new URLSearchParams(window.location.search).has(
      'quickSearch',
    );
    void app.start().then(() => {
      const id = new URLSearchParams(window.location.search).get('script');
      if (id && app.nodes.some((n) => n.node.id === id)) app.select(id, true);
    });
    const stopOpen = native()
      ? listen<string>('open-script', async (event) => {
          await app.refresh();
          if (
            event.payload &&
            app.nodes.some((n) => n.node.id === event.payload)
          )
            app.select(event.payload, true);
        })
      : null;
    let polling = false;
    const timer = setInterval(async () => {
      if (polling || !app.data) return;
      polling = true;
      try {
        app.progress = await api<SyncStatus>('/sync/status');
        if (
          app.progress.phase === 'current' &&
          app.progress.version !== app.workspace?.version &&
          app.nav !== 'scripts'
        )
          await app.refresh();
      } catch {
        /* Existing local content stays usable; initial connection errors are shown by start(). */
      } finally {
        polling = false;
      }
    }, 2000);
    return () => {
      clearInterval(timer);
      void stopOpen?.then((stop) => stop());
    };
  });
  function navigate(id: string) {
    return app.navigate(id);
  }
  function shortcut(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      searchInput?.focus();
    }
    if (event.key === 'Escape' && app.query) app.query = '';
  }
</script>

<svelte:head
  ><title>KnowToSay · 话术与知识库</title><meta
    name="description"
    content="可离线使用的销售话术与知识库工作台"
  /></svelte:head
>
<svelte:window onkeydown={shortcut} />
{#if !quickSearchMode}
  <header class="global-nav" aria-label="应用信息">
    <span>KnowToSay</span>
    <span
      >{app.workspace
        ? `${app.workspace.name} · ${app.admin ? '管理模式' : '使用模式'}`
        : '话术与知识库'}</span
    >
  </header>
{/if}
{#if quickSearchMode}<QuickSearch />
{:else if app.loading}<div class="boot-screen">
    <div class="brand-mark">K<span></span></div>
    <LoaderCircle class="spin" size={22} />
    <p>正在打开本地资料库…</p>
  </div>
{:else if !app.data}<div class="boot-screen">
    <div class="brand-mark">K<span></span></div>
    <h1>连接本地资料库</h1>
    <p class="muted">请启动 KnowToSay 本地服务后重试。</p>
    <button class="button primary" onclick={() => app.start()}
      ><RefreshCw size={16} />重新连接</button
    >
  </div>
{:else if !app.workspace}<Onboarding />
{:else}
  <div class:compact={app.compact} class="app-shell">
    {#if !app.compact}<aside class="sidebar">
        <a
          class="brand"
          href="/"
          onclick={(event) => {
            event.preventDefault();
            navigate('workbench');
          }}
          ><div class="brand-mark">K<span></span></div>
          <div>KnowToSay<small>让知识成为好回答</small></div></a
        >
        <div class="workspace-switch">
          <div class="workspace-avatar">{app.workspace.name.slice(0, 1)}</div>
          <span
            >{app.workspace.name}<small
              >{app.admin ? '管理员工作空间' : '业务员工作空间'}</small
            ></span
          ><ChevronDown size={15} />
        </div>
        <span class="nav-caption">工作空间</span>
        <nav>
          {#each navigation as item}{@const Icon = item.icon}<button
              class:active={app.nav === item.id && !app.query}
              onclick={() => navigate(item.id)}
              ><Icon size={19} strokeWidth={1.65} /><span
                >{item.id === 'sync' && !app.admin
                  ? '资料同步'
                  : item.name}</span
              >{#if item.id === 'scripts'}<small>{app.nodes.length}</small
                >{/if}</button
            >{/each}
        </nav>
        <div class="sidebar-bottom">
          <div class="local-note">
            <span class="status-dot"></span><strong>本地资料库</strong><span
              >v{app.workspace.version}</span
            >
          </div>
          <p>
            {app.admin
              ? '编辑完成后，发布给团队。'
              : '个人话术独立保存，离线可用。'}
          </p>
          <div class="sidebar-signature">
            K<span> / </span>KNOW. SAY. CONNECT.
          </div>
        </div>
      </aside>{/if}
    <div class="main-shell">
      <header class="topbar">
        <div class="global-search">
          <Search size={18} /><input
            bind:this={searchInput}
            bind:value={app.query}
            disabled={app.nav === 'scripts' && app.editorDirty}
            placeholder={app.compact
              ? '搜索话术、素材…'
              : '搜索话术、素材、Excel 内容…'}
            aria-label="全局搜索"
          />{#if app.query}<button
              class="icon-button"
              aria-label="清空搜索"
              onclick={() => (app.query = '')}><X size={15} /></button
            >{:else if !app.compact}<kbd>Ctrl K</kbd>{/if}
        </div>
        <div class="topbar-actions">
          {#if app.compact}<label class="compact-page-picker">
              <span class="visually-hidden">页面导航</span>
              <select
                aria-label="悬浮窗导航"
                value={app.nav}
                onchange={(event) => {
                  if (!navigate(event.currentTarget.value))
                    event.currentTarget.value = app.nav;
                }}
              >
                {#each navigation as item}<option value={item.id}
                    >{item.name}</option
                  >{/each}
              </select>
            </label>{/if}
          <label class="language-picker"
            ><span>{app.compact ? '' : '发送语言'}</span><select
              bind:value={app.language}
              aria-label="发送语言"
              >{#each app.languages as language}<option value={language}
                  >{languageName(language)}</option
                >{/each}</select
            ></label
          ><span class="topbar-divider"></span><button
            class="icon-button window-toggle"
            aria-label={app.compact ? '切换完整窗口' : '切换悬浮窗口'}
            title={app.compact
              ? '切换完整窗口'
              : native()
                ? '切换置顶悬浮窗口'
                : '预览精简布局'}
            disabled={!!app.busy}
            onclick={() => app.toggleCompact()}
            >{#if app.compact}<Expand size={19} />{:else}<PanelRightClose
                size={19}
              />{/if}</button
          >
        </div>
      </header>
      <main class:workbench-main={app.nav === 'workbench' && !app.query}>
        {#if app.compact}<div class="compact-heading">
            <strong
              >{navigation.find((item) => item.id === app.nav)?.name ||
                'KnowToSay'}</strong
            >
            <span>{native() ? '置顶悬浮窗' : '精简布局预览'}</span>
          </div>{/if}
        {#if app.query.trim()}<SearchResults />
        {:else}
          {#if !app.compact && app.nav === 'workbench'}<div
              class="workbench-heading"
            >
              <div>
                <span class="eyebrow">YOUR CONVERSATION COMPANION</span>
                <h1>让每次沟通，<span>都有准备。</span></h1>
                <p>从一句问候，到恰好的回应。</p>
              </div>
              <div class="workspace-metrics">
                <div>
                  <strong>{app.nodes.length}</strong><span>可用话术</span>
                </div>
                <div>
                  <strong>{app.materials.length}</strong><span>知识素材</span>
                </div>
              </div>
            </div>{/if}
          {#if app.nav === 'workbench'}<Workbench />
          {:else if app.nav === 'scripts'}<ScriptsPage />
          {:else if app.nav === 'library'}<LibraryPage />
          {:else if app.nav === 'sync'}<SyncPage />
          {:else}<SettingsPage />{/if}
        {/if}
      </main>
      <footer class="app-footer">
        <span
          ><span class="status-dot"></span>{app.admin
            ? '本机主库'
            : app.progress?.phase === 'failed'
              ? '离线使用本地资料'
              : '本地资料可用'}<span class="footer-separator">/</span>{app
            .workspace.name}</span
        ><span
          >{app.busy ||
            (app.admin && app.data.draftDirty
              ? '有未发布的草稿'
              : app.progress?.message ||
                `已发布版本 v${app.workspace.version}`)}</span
        >
      </footer>
    </div>
  </div>
{/if}
{#if app.error}<div class="notification error" role="alert">
    <span>{app.error}</span><button
      class="icon-button"
      aria-label="关闭错误提示"
      onclick={() => (app.error = '')}><X size={17} /></button
    >
  </div>{/if}
{#if app.notice}<div class="notification success" role="status">
    <Check size={18} /><span>{app.notice}</span><button
      class="icon-button"
      aria-label="关闭提示"
      onclick={() => (app.notice = '')}><X size={15} /></button
    >
  </div>{/if}
{#if app.selectedMaterialId}<MaterialDialog />{/if}
