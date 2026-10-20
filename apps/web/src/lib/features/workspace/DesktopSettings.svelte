<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { Keyboard, ScanLine } from 'lucide-svelte';
  import { native } from './api';
  import { shortcutFromKey } from './shortcut';
  import { app } from './store.svelte';
  import type { DesktopStatus } from '$lib/types';
  let recording = $state(false);
  function capture(event: KeyboardEvent) {
    if (!recording) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    if (event.key === 'Escape') {
      recording = false;
      return;
    }
    const shortcut = shortcutFromKey(event);
    if (shortcut && status) {
      status.preferences.shortcut = shortcut;
      recording = false;
    }
  }
  let status = $state<DesktopStatus | null>(null);
  onMount(() => {
    if (native())
      invoke<DesktopStatus>('desktop_status')
        .then((value) => (status = value))
        .catch((e) => (app.error = String(e)));
  });
  function toggle(edge: string, checked: boolean) {
    if (status)
      status.preferences.edges = checked
        ? [...status.preferences.edges, edge]
        : status.preferences.edges.filter((e) => e !== edge);
  }
  async function save() {
    if (!status) return;
    await app.task(
      '保存桌面设置',
      async () => {
        status = await invoke<DesktopStatus>('save_desktop_preferences', {
          preferences: status!.preferences,
        });
      },
      '桌面唤出设置已保存',
    );
  }
</script>

<svelte:window onkeydowncapture={capture} />
<div class="settings-card">
  <span class="card-icon"><Keyboard size={24} /></span>
  <h3>托盘与快捷搜索</h3>
  <p class="muted">
    最小化或关闭工作台后常驻托盘。边缘与快捷键唤出同一个搜索栏；边缘默认选中快捷话术，快捷键默认选中全局搜索，可切换流程、快捷话术或知识库范围，保留原来的话术流程位置。
  </p>
  {#if !native()}<p class="field-help">
      这些功能在桌面客户端中启用。可在浏览器预览独立搜索界面。
    </p>
    <a
      class="button secondary"
      href="/?quickSearch=1"
      target="_blank"
      rel="noreferrer">预览独立搜索栏</a
    ><a
      class="button secondary"
      href="/?quickSearch=1&scope=quick"
      target="_blank"
      rel="noreferrer">预览边缘唤出搜索</a
    >{:else if status}<label
      >全局快捷键（留空关闭）<input
        bind:value={status.preferences.shortcut}
        placeholder="Ctrl+Alt+Space"
      /></label
    >
    <div class="button-row">
      <button class="button secondary" onclick={() => (recording = !recording)}
        >{recording ? '请按新的组合键（Esc 取消）' : '录入快捷键'}</button
      ><button
        class="text-button"
        onclick={() => {
          if (status) status.preferences.shortcut = 'Ctrl+Alt+Space';
        }}>恢复默认快捷键</button
      >
    </div>
    <p class="field-help">
      {status.shortcutRegistered
        ? '当前系统快捷键已注册'
        : '系统快捷键尚未启用'}。修改后点击“保存唤出设置”。留空可关闭全局快捷键；Ctrl+K
      仅用于聚焦当前窗口搜索框。
    </p>
    {#if status.warning}<p class="info-note">{status.warning}</p>{/if}
    <div class="desktop-edges">
      <span class="section-label"
        >工作台已收起时，在哪些边缘停留唤出快捷话术搜索</span
      >
      <div class="button-row">
        {#each [['left', '左边缘'], ['right', '右边缘'], ['top', '上边缘'], ['bottom', '下边缘']] as [edge, label]}<label
            class="check-label"
            ><input
              type="checkbox"
              checked={status.preferences.edges.includes(edge)}
              onchange={(e) => toggle(edge, e.currentTarget.checked)}
            />{label}</label
          >{/each}
      </div>
    </div>
    <label
      >停留时间（毫秒）<input
        type="number"
        min="200"
        max="2000"
        step="50"
        bind:value={status.preferences.dwellMs}
      /></label
    ><label class="check-label"
      ><input
        type="checkbox"
        bind:checked={status.preferences.copyHides}
      />复制成功后收起搜索栏</label
    >
    <p class="field-help">
      默认右边缘、400
      毫秒。拖拽鼠标时和其他全屏应用中暂停边缘唤出；上下左右可分别开启。
    </p>
    <button class="button primary" disabled={!!app.busy} onclick={save}
      >保存唤出设置</button
    >{/if}
</div>
