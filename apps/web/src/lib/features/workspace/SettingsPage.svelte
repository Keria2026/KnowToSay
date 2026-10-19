<script lang="ts">
  import { HardDrive, FolderOpen, RotateCcw, Copy, Check } from 'lucide-svelte';
  import { app } from './store.svelte';
  import { api, chooseDirectory, native } from './api';
  import type { FileResult, Snapshot } from '$lib/types';
  import SyncSettings from './SyncSettings.svelte';
  import DesktopSettings from './DesktopSettings.svelte';
  let backupDirectory = $state('');
  let restoreDirectory = $state('');
  let importDirectory = $state('');
  async function importBackup() {
    const directory = native() ? await chooseDirectory() : importDirectory;
    if (native() && !directory) return;
    if (!directory) {
      app.error = '请选择备份文件夹';
      return;
    }
    if (
      !confirm(
        '将此工作组备份中的草稿资料追加到当前主库？当前资料会保留，导入内容需发布后才同步给业务员。',
      )
    )
      return;
    await app.task(
      '正在校验并导入工作组备份',
      async () => {
        app.data = await api<Snapshot>('/backup/import', 'POST', { directory });
      },
      '已导入到当前主库草稿，请检查后发布',
    );
  }
  let result = $state('');
  async function backup() {
    const directory = native() ? await chooseDirectory() : backupDirectory;
    if (native() && !directory) return;
    await app.task(
      '正在备份，请等待',
      async () => {
        result = (
          await api<FileResult>('/backup', 'POST', {
            directory: directory || null,
          })
        ).path;
      },
      '备份已完成并校验',
    );
  }
  async function restore() {
    const directory = native() ? await chooseDirectory() : restoreDirectory;
    if (!directory) {
      app.error = '请选择备份文件夹';
      return;
    }
    if (
      !confirm(
        '用此备份恢复当前工作组？当前主库草稿将被替换。建议先备份当前数据。',
      )
    )
      return;
    await app.task(
      '正在校验并恢复',
      async () => {
        app.data = await api<Snapshot>('/restore', 'POST', { directory });
      },
      '已恢复同一工作组，并创建新的发布版本供业务员更新',
    );
  }
  async function personalBackup() {
    const directory = native() ? await chooseDirectory() : backupDirectory;
    if (native() && !directory) return;
    await app.task(
      '备份个人话术',
      async () => {
        result = (
          await api<FileResult>('/personal/export', 'POST', {
            directory: directory || null,
          })
        ).path;
      },
      '个人话术已备份',
    );
  }
  async function personalRestore() {
    const directory = native() ? await chooseDirectory() : restoreDirectory;
    if (
      !directory ||
      !confirm(
        '用备份替换当前个人分支？管理员版本和知识库保持不变。建议先备份当前个人内容。',
      )
    )
      return;
    await app.task(
      '恢复个人话术',
      async () => {
        app.data = await api<Snapshot>('/personal/import', 'POST', {
          directory,
        });
      },
      '已恢复个人分支',
    );
  }
</script>

<section class="settings-page">
  <div class="page-heading">
    <div>
      <span class="eyebrow">YOUR LOCAL WORKSPACE</span>
      <h2>设置</h2>
      <p class="muted">管理发布、同步、工作组和本地备份。</p>
    </div>
  </div>
  <SyncSettings />
  <div class="settings-card">
    <h3>当前工作组</h3>
    <dl class="workspace-details">
      <div>
        <dt>名称</dt>
        <dd>{app.workspace?.name}</dd>
      </div>
      <div>
        <dt>此电脑的角色</dt>
        <dd>{app.admin ? '管理员' : '业务员'}</dd>
      </div>
      <div>
        <dt>工作组标识</dt>
        <dd>
          <code>{app.workspace?.groupId}</code><button
            class="icon-button"
            aria-label="复制工作组标识"
            onclick={() => app.copy(app.workspace?.groupId || '')}
            ><Copy size={14} /></button
          >
        </dd>
      </div>
      <div>
        <dt>已发布版本</dt>
        <dd>v{app.workspace?.version || 0}</dd>
      </div>
    </dl>
    <p class="field-help">
      身份在初始化后持久保存。管理员离线不会让其他电脑自动成为管理员。
    </p>
  </div>
  {#if app.admin}<div class="settings-card">
      <span class="card-icon"><HardDrive size={23} /></span>
      <h3>备份管理员资料</h3>
      <p class="muted">
        包含主库草稿、已发布资料、素材文件和工作组身份，逐文件校验后生成完整备份文件夹。
      </p>
      {#if !native()}<label
          >备份父文件夹（留空使用本地导出目录）<input
            bind:value={backupDirectory}
            placeholder="请选择独立磁盘上的目录"
          /></label
        >{/if}<button
        class="button primary"
        disabled={!!app.busy}
        onclick={backup}><FolderOpen size={16} />创建备份</button
      >{#if result}<div class="result-path">
          <Check size={16} />备份已完成：<code>{result}</code>
        </div>{/if}
      <p class="field-help">
        建议把备份放到独立磁盘。此备份不包含其他电脑的个人话术。
      </p>
    </div>
    <div class="settings-card">
      <h3>恢复同一工作组</h3>
      <p class="muted">
        恢复前检查所有素材，校验失败不替换当前主库。其他工作组的资料请使用下方“导入工作组备份”。
      </p>
      {#if !native()}<label
          >备份文件夹<input
            bind:value={restoreDirectory}
            placeholder="包含 backup.json 的文件夹绝对路径"
          /></label
        >{/if}<button
        class="button secondary"
        disabled={!!app.busy}
        onclick={restore}><RotateCcw size={16} />选择备份并恢复</button
      >
    </div>
    <div class="settings-card">
      <h3>导入工作组备份</h3>
      <p class="muted">
        将其他管理员备份中的草稿话术、流程、知识库素材和分类追加到自己的主库。当前资料会保留，导入后检查并发布。
      </p>
      <p class="field-help">
        当前工作组名称、管理员身份和已发布版本保持不变。同名流程会自动区分；备份内的连线、素材关联和排版一并导入。
      </p>
      {#if !native()}<label
          >要导入的工作组备份文件夹<input
            bind:value={importDirectory}
            placeholder="包含 backup.json 的文件夹绝对路径"
          /></label
        >{/if}
      <button
        class="button secondary"
        disabled={!!app.busy}
        onclick={importBackup}
        ><FolderOpen size={16} />选择工作组备份并导入</button
      >
    </div>
  {:else}<div class="settings-card">
      <h3>备份个人话术</h3>
      <p class="muted">备份个人话术与流程，不复制统一管理的知识库素材。</p>
      {#if !native()}<label
          >备份父文件夹（可留空）<input
            bind:value={backupDirectory}
            placeholder="本机绝对路径"
          /></label
        >{/if}<button
        class="button primary"
        disabled={!!app.busy}
        onclick={personalBackup}>导出个人备份</button
      >{#if result}<div class="result-path">
          <Check size={16} /><code>{result}</code>
        </div>{/if}
    </div>
    <div class="settings-card">
      <h3>恢复个人分支</h3>
      <p class="muted">
        只接受当前工作组的个人备份，不覆盖管理员版本或知识库。
      </p>
      {#if !native()}<label
          >个人备份文件夹<input
            bind:value={restoreDirectory}
            placeholder="包含 personal.json 的文件夹"
          /></label
        >{/if}<button
        class="button secondary"
        disabled={!!app.busy}
        onclick={personalRestore}><RotateCcw size={16} />恢复个人备份</button
      >
    </div>{/if}
  <DesktopSettings />
</section>
