<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import {
    Search,
    MessageSquare,
    GitBranch,
    BookOpen,
    Settings,
    PanelRightClose,
    Expand,
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
  import SettingsPage from '$lib/features/workspace/SettingsPage.svelte';
  import CompactWorkbench from '$lib/features/scripts/CompactWorkbench.svelte';
  import ReplyDialog from '$lib/features/scripts/ReplyDialog.svelte';
  import SearchScopePicker from '$lib/features/workspace/SearchScopePicker.svelte';
  import QuickSearch from '$lib/features/workspace/QuickSearch.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  let quickSearchMode = $state(false);
  let searchInput = $state<HTMLInputElement>();
  const navigation = [
    { id: 'workbench', name: '销售工作台', icon: MessageSquare },
    { id: 'scripts', name: '话术与流程', icon: GitBranch },
    { id: 'library', name: '知识库', icon: BookOpen },
    { id: 'settings', name: '设置', icon: Settings },
  ];
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
      if (app.compact)
        document
          .querySelector<HTMLInputElement>('.compact-search input')
          ?.focus();
      else searchInput?.focus();
    }
    if (!app.compact && event.key === 'Escape' && app.query) app.query = '';
  }
</script>

<svelte:head
  ><title>KnowToSay · 话术与知识库</title><meta
    name="description"
    content="可离线使用的销售话术与知识库工作台"
  /></svelte:head
>
<svelte:window onkeydown={shortcut} />
{#if quickSearchMode}<QuickSearch />
{:else if app.loading}<div class="boot-screen">
    <img class="brand-mark" src="/icon.svg" alt="KnowToSay" />
    <LoaderCircle class="spin" size={22} />
    <p>正在打开本地资料库…</p>
  </div>
{:else if !app.data}<div class="boot-screen">
    <img class="brand-mark" src="/icon.svg" alt="KnowToSay" />
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
          ><img class="brand-mark" src="/icon.svg" alt="KnowToSay" />
          <div>KnowToSay<small>让知识成为好回答</small></div></a
        >
        <span class="nav-caption">工作空间</span>
        <nav>
          {#each navigation as item}{@const Icon = item.icon}<button
              aria-label={item.name}
              title={item.name}
              class:active={app.nav === item.id && !app.query}
              aria-current={app.nav === item.id && !app.query
                ? 'page'
                : undefined}
              onclick={() => navigate(item.id)}
              ><Icon size={19} strokeWidth={1.65} /><span>{item.name}</span
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
    <div
      class="main-shell"
      class:full-shell-hidden={app.compact}
      inert={app.compact}
    >
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
        <SearchScopePicker bind:scope={app.searchScope} />
        <div class="topbar-actions">
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
    {#if app.compact}<CompactWorkbench />{/if}
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

{#if app.replyId}<ReplyDialog />{/if}
