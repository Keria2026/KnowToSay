<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { X, Copy, ArrowRight } from 'lucide-svelte';
  import type { NodeRecord } from '$lib/types';
  import { app } from '../workspace/store.svelte';
  import { native } from '../workspace/api';
  import MaterialResult from '../knowledge/MaterialResult.svelte';
  import { languageName, replyPreview } from '$lib/utils/scripts';
  import FlowCanvas from './FlowCanvas.svelte';

  let {
    records,
    selectedId,
    name,
    onclose,
    oncontinue = (id: string) => app.select(id, true),
    continueLabel = '在工作台继续此步骤',
  }: {
    records: NodeRecord[];
    selectedId: string;
    name: string;
    onclose: () => void;
    oncontinue?: (id: string) => void;
    continueLabel?: string;
  } = $props();
  let selected = $state(untrack(() => selectedId));
  let node = $derived(
    records.find((record) => record.node.id === selected)?.node,
  );
  let preview = $derived(
    node ? replyPreview(node, app.language, app.values) : null,
  );
  let dialog: HTMLDialogElement;
  let screenError = $state('');
  let closing = false;
  let restore: (() => Promise<void>) | undefined;

  async function enterScreen() {
    try {
      if (native()) {
        const window = getCurrentWindow();
        const wasFullscreen = await window.isFullscreen();
        if (!wasFullscreen) {
          await window.setFullscreen(true);
          restore = () => window.setFullscreen(false);
        }
      } else if (!document.fullscreenElement) {
        await document.documentElement.requestFullscreen();
        restore = async () => {
          if (document.fullscreenElement === document.documentElement)
            await document.exitFullscreen();
        };
      }
    } catch {
      screenError = '未能进入屏幕全屏，当前以窗口内全屏显示。';
    }
    if (closing) await restore?.();
  }

  async function close() {
    if (closing) return;
    closing = true;
    try {
      await restore?.();
      onclose();
    } catch {
      closing = false;
      screenError = '退出全屏失败，请重试。';
    }
  }

  onMount(() => {
    const previous = document.activeElement as HTMLElement | null;
    dialog.showModal();
    // Called during the opening click so browsers retain user activation.
    void enterScreen();
    const fullscreenChanged = () => {
      if (restore && !document.fullscreenElement) void close();
    };
    if (!native())
      document.addEventListener('fullscreenchange', fullscreenChanged);
    return () => {
      document.removeEventListener('fullscreenchange', fullscreenChanged);
      closing = true;
      void restore?.().catch(() => {});
      previous?.focus();
    };
  });
</script>

<dialog
  bind:this={dialog}
  class="flow-fullscreen"
  aria-label="全屏流程查看"
  oncancel={(event) => {
    event.preventDefault();
    void close();
  }}
  onkeydown={(event) => {
    if (event.key === 'Escape' || event.key === 'F11') {
      event.preventDefault();
      event.stopPropagation();
      void close();
    }
  }}
>
  <div class="fullscreen-flow-toolbar">
    <div>
      <strong>{name}</strong><span
        >{records.length} 个步骤 · 点击节点查看详情</span
      >
    </div>
    <button class="button secondary" onclick={close}
      ><X size={17} />退出全屏</button
    >
  </div>
  {#if screenError}<p class="fullscreen-flow-error" role="status">
      {screenError}
    </p>{/if}
  <div class="fullscreen-flow-body">
    <div class="fullscreen-flow-canvas">
      <FlowCanvas
        {records}
        selectedId={selected}
        label="全屏流程图"
        onselect={(id) => (selected = id)}
      />
    </div>
    {#if node}<aside class="flow-node-inspector" aria-label="节点详情">
        <div class="section-label">
          话术详情 <span>{node.isEntry ? '流程起点' : '流程步骤'}</span>
        </div>
        <h2>{node.title}</h2>
        {#if node.question}<p class="question">{node.question}</p>{/if}
        {#each ['zh-CN', 'ko'] as language}
          <section class="flow-inspector-reply">
            <div class="section-label">{languageName(language)}</div>
            <p>{node.replies[language] || '尚未维护此语言版本'}</p>
          </section>
        {/each}
        {#if node.variables.length}<div class="variables">
            <div class="section-label">填写回复变量</div>
            {#each node.variables as variable}<label>
                <span
                  >{variable.label || variable.name}{variable.required
                    ? ' *'
                    : ''}</span
                >
                <input
                  aria-label={variable.label || variable.name}
                  bind:value={app.values[variable.name]}
                />
              </label>{/each}
          </div>{/if}
        <label class="flow-inspector-language"
          >发送语言<select aria-label="全屏发送语言" bind:value={app.language}>
            {#each app.languages as language}<option value={language}
                >{languageName(language)}</option
              >{/each}
          </select></label
        >
        <section class="flow-inspector-reply">
          <div class="section-label">复制预览</div>
          <p>
            {preview?.languageMissing ? '尚未维护此语言版本' : preview?.text}
          </p>
          {#if preview?.missing.length}<p class="field-help">
              待填写：{preview.missing.join('、')}
            </p>{/if}
          <button
            class="button primary"
            disabled={!preview?.ready || !!app.busy}
            onclick={() => preview?.ready && app.copy(preview.text)}
            ><Copy size={16} />复制回复</button
          >
        </section>
        {#if node.branches.length}<div class="next-step">
            <div class="section-label">下一步</div>
            <div class="branch-buttons">
              {#each node.branches as branch}<button
                  disabled={!records.some((r) => r.node.id === branch.targetId)}
                  onclick={() => (selected = branch.targetId)}
                  >{branch.label}<ArrowRight size={16} /></button
                >{/each}
            </div>
          </div>{/if}
        <div class="section-label">关联素材 · {node.materialIds.length}</div>
        {#each node.materialIds as id}{@const material = app.materials.find(
            (m) => m.id === id,
          )}
          {#if material}<MaterialResult hit={{ material, rows: [] }} />{:else}<p
              class="field-help"
            >
              素材已移除
            </p>{/if}
        {/each}
        <button
          class="button secondary"
          onclick={async () => {
            if (node) oncontinue(node.id);
            await close();
          }}>{continueLabel}</button
        >
      </aside>{/if}
  </div>
</dialog>
