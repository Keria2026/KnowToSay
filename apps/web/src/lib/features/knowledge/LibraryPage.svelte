<script lang="ts">
  import { tick } from 'svelte';
  import {
    Plus,
    Upload,
    FileText,
    Image,
    Film,
    FileSpreadsheet,
    Pencil,
    Trash2,
    X,
    ArrowRight,
    Download,
    Check,
    FolderPlus,
    Folder as FolderIcon,
  } from 'lucide-svelte';
  import { languageName } from '$lib/utils/scripts';
  import { app } from '../workspace/store.svelte';
  import {
    api,
    native,
    selectNativeFiles,
    uploadFiles,
  } from '../workspace/api';
  import { kindName, bytes } from '$lib/utils/scripts';
  import type { Material, Snapshot, ScriptNode } from '$lib/types';
  import MaterialImage from './MaterialImage.svelte';
  import FolderDialog from './FolderDialog.svelte';
  import CaseEditor from './CaseEditor.svelte';
  import { folderPath, folderOf, libraryItems } from './folders';
  import type { Folder } from '$lib/types';
  let kind = $state('all');
  let folderId = $state<string | null>(null);
  let folderDialog = $state<{ folder?: Folder } | null>(null);
  let caseDialog = $state<{ material?: Material } | null>(null);
  let childFolders = $derived(
    app.folders.filter((f) => f.parentId === folderId),
  );
  let currentFolder = $derived(app.folders.find((f) => f.id === folderId));
  let fileInput = $state<HTMLInputElement>();
  let editDialog: HTMLDialogElement;
  let importDialog: HTMLDialogElement;
  let editing = $state<Material | null>(null);
  let tags = $state('');
  let source = $state<Material | null>(null);
  let mapping = $state<Record<string, string>>({});
  let importFlowId = $state('');
  let preview = $state<ScriptNode[]>([]);
  let previewSignature = $state('');
  let visible = $derived(
    libraryItems(app.materials).filter(
      (m) =>
        (kind === 'all' || m.kind === kind) &&
        folderOf(m, app.folders) === folderId,
    ),
  );
  const fields: [string, string][] = [
    ['id', '唯一 ID（必填）'],
    ['title', '题目（必填）'],
    ['question', '问题 / 说明'],
    ['category', '快捷话术分类'],
    ['kind', '话术类型'],
    ['isEntry', '流程入口'],
    ['reply:zh-CN', '中文回复'],
    ['reply:ko', '韩语回复'],
    ['reply:en', '英语回复'],
    ['variables', '变量 JSON'],
    ['branches', '分支 JSON'],
    ['materialIds', '关联素材 ID JSON'],
  ];
  const icon = (type: string) =>
    ({ image: Image, video: Film, excel: FileSpreadsheet, text: FileText })[
      type
    ] || FileText;
  async function importFiles(files?: FileList | null) {
    const targetFolder = folderId;
    await app.task(
      '正在导入文件',
      async () => {
        const imported = native()
          ? await selectNativeFiles()
          : files?.length
            ? await uploadFiles(files)
            : [];
        if (targetFolder !== null) {
          for (const material of imported) {
            await api('/materials', 'POST', {
              ...material,
              category: '',
              folderId: targetFolder,
            });
          }
        }
        await app.refresh();
      },
      '文件已导入到管理员草稿',
    );
    if (fileInput) fileInput.value = '';
  }
  async function edit(material?: Material) {
    if (material?.kind === 'case') {
      caseDialog = { material };
      return;
    }
    editing = material
      ? JSON.parse(JSON.stringify(material))
      : {
          id: '',
          name: '',
          kind: 'text',
          category: '',
          folderId,
          imageIds: [],
          tags: [],
          notes: '',
          text: '',
          mime: 'text/plain',
          hash: null,
          size: 0,
          rows: [],
        };
    tags = editing!.tags.join('，');
    await tick();
    editDialog.showModal();
  }
  async function save() {
    if (!editing) return;
    const material = {
      ...editing,
      category: '',
      tags: tags
        .split(/[,，]/)
        .map((t) => t.trim())
        .filter(Boolean),
    };
    await app.task(
      '保存素材',
      async () => {
        app.data = await api<Snapshot>('/materials', 'POST', material);
        editDialog.close();
      },
      '已保存到草稿，发布后同步给业务员',
    );
  }
  async function remove(material: Material) {
    if (
      !confirm(
        `从草稿中移除「${material.name}」？发布并同步后，个人话术中的引用会显示为已移除。`,
      )
    )
      return;
    await app.task('移除素材', async () => {
      app.data = await api<Snapshot>(
        `/materials/${encodeURIComponent(material.id)}`,
        'DELETE',
      );
    });
  }
  async function openImport(material: Material) {
    source = material;
    importFlowId = app.activeFlowId;
    preview = [];
    previewSignature = '';
    const header = material.rows[0]?.cells || [];
    mapping = Object.fromEntries(
      fields.map(([key]) => [key, header.includes(key) ? key : '']),
    );
    await tick();
    importDialog.showModal();
  }
  const cleanMapping = () =>
    Object.fromEntries(Object.entries(mapping).filter(([, v]) => v));
  async function parseImport(commit = false) {
    if (!source) return;
    await app.task(commit ? '导入话术' : '校验表格', async () => {
      preview = await api<ScriptNode[]>('/scripts/import', 'POST', {
        materialId: source!.id,
        flowId: importFlowId || null,
        mapping: cleanMapping(),
        preview: !commit,
      });
      previewSignature = JSON.stringify({ mapping, importFlowId });
      if (commit) {
        await app.refresh();
        importDialog.close();
        app.notify(`已导入 ${preview.length} 条话术到草稿`);
      }
    });
  }
</script>

<section class="library-page">
  <div class="page-heading">
    <div>
      <span class="eyebrow">KNOWLEDGE LIBRARY</span>
      <h2>知识库</h2>
      <p class="muted">把每份资料，放在需要它的地方。</p>
    </div>
    {#if app.admin}<div class="button-row">
        <button class="button secondary" onclick={() => (folderDialog = {})}
          ><FolderPlus size={16} />新建文件夹</button
        >
        <button class="button secondary" onclick={() => (caseDialog = {})}
          ><Plus size={16} />上传案例</button
        >
        <button class="button secondary" onclick={() => edit()}
          ><Plus size={16} />文字素材</button
        ><button
          class="button primary"
          disabled={!!app.busy}
          onclick={() => (native() ? importFiles() : fileInput?.click())}
          ><Upload size={16} />导入文件</button
        >
      </div>
      <input
        class="visually-hidden"
        type="file"
        multiple
        bind:this={fileInput}
        onchange={(event) => importFiles(event.currentTarget.files)}
      />{/if}
  </div>
  <nav class="folder-breadcrumbs" aria-label="文件夹导航">
    <button class="text-button" onclick={() => (folderId = null)}
      >资料根目录</button
    >
    {#if currentFolder}<span>/ {folderPath(app.folders, folderId)}</span><button
        class="text-button"
        onclick={() => (folderId = currentFolder?.parentId ?? null)}
        >返回上一级</button
      >
      {#if app.admin}<button
          class="text-button"
          onclick={() => (folderDialog = { folder: currentFolder })}
          >管理当前文件夹</button
        ><button
          class="text-button"
          onclick={async () => {
            if (
              currentFolder &&
              confirm(
                '删除此文件夹？其中素材和子文件夹将移到上一级，资料不会删除。',
              )
            ) {
              const parent = currentFolder.parentId ?? null;
              await app.task('删除文件夹', async () => {
                app.data = await api<Snapshot>(
                  `/folders/${encodeURIComponent(folderId!)}`,
                  'DELETE',
                );
                folderId = parent;
              });
            }
          }}>删除当前文件夹</button
        >{/if}
    {/if}
  </nav>
  <div class="folder-grid">
    {#each childFolders as folder (folder.id)}<button
        class="folder-card"
        onclick={() => (folderId = folder.id)}
        ><FolderIcon size={30} /><strong>{folder.name}</strong></button
      >{/each}
  </div>
  <div class="library-toolbar">
    <div class="filter-tabs">
      {#each [['all', '全部'], ['case', '案例'], ['text', '文字'], ['image', '图片'], ['video', '视频'], ['excel', '表格'], ['file', '文件']] as [key, label]}<button
          class:active={kind === key}
          onclick={() => (kind = key)}>{label}</button
        >{/each}
    </div>
  </div>
  <div class="library-summary">
    <span>{visible.length} 份素材</span><span
      >{bytes(visible.reduce((sum, m) => sum + m.size, 0))}</span
    >{#if !app.admin}<span>管理员统一维护 · 本机可离线使用</span>{/if}
  </div>
  <div class="material-grid">
    {#each visible as material}{@const Icon = icon(material.kind)}
      <article class="material-card">
        <button
          class="material-card-main"
          onclick={() => (app.selectedMaterialId = material.id)}
          ><div
            class="material-cover"
            class:video-cover={material.kind === 'video'}
          >
            {#if material.kind === 'case' && material.imageIds?.length}{@const cover =
                app.materials.find(
                  (m) => m.id === material.imageIds?.[0],
                )}{#if cover}<MaterialImage
                  material={cover}
                />{/if}{:else if material.kind === 'image'}<MaterialImage
                {material}
              />{:else}<Icon size={35} strokeWidth={1.25} />{/if}<span
              >{kindName(material.kind)}{material.kind === 'case'
                ? ` · ${material.imageIds?.length ?? 0} 张图片`
                : ''}</span
            >
          </div>
          <div class="material-card-body">
            <span class="eyebrow"
              >{folderPath(app.folders, folderOf(material, app.folders))}</span
            >
            <h3>{material.name}</h3>
            <p>
              {material.hash
                ? `${bytes(material.size)} · 本地素材`
                : material.kind === 'case'
                  ? `${material.imageIds?.length ?? 0} 张图片 · ${bytes(material.size)}`
                  : '文字素材'}
            </p>
            <div class="tag-list">
              {#each material.tags.slice(0, 3) as tag}<span class="tag"
                  >{tag}</span
                >{/each}
            </div>
          </div></button
        >{#if app.admin}<footer class="material-card-actions">
            {#if material.kind === 'excel'}<button
                class="text-button"
                onclick={() => openImport(material)}
                ><FileSpreadsheet size={14} />导入为话术</button
              >{:else}<span></span>{/if}
            <div class="inline-actions">
              <button
                class="icon-button"
                aria-label={`编辑${material.name}`}
                onclick={() => edit(material)}><Pencil size={15} /></button
              ><button
                class="icon-button"
                aria-label={`移除${material.name}`}
                onclick={() => remove(material)}><Trash2 size={15} /></button
              >
            </div>
          </footer>{/if}
      </article>{/each}
  </div>
  {#if !visible.length}<div class="empty">
      <FileText size={38} />
      <h3>{folderId === null ? '这里还没有素材' : '此文件夹还没有素材'}</h3>
      <p>
        {app.admin
          ? '上传案例、导入文件或创建文字素材。'
          : '请等待管理员发布资料，或检查当前筛选条件。'}
      </p>
    </div>{/if}
</section>

<dialog
  bind:this={editDialog}
  class="edit-material-dialog"
  onclose={() => (editing = null)}
>
  <header class="dialog-header">
    <h2>{editing?.id ? '编辑素材' : '新建文字素材'}</h2>
    <button
      class="icon-button"
      aria-label="关闭编辑"
      onclick={() => editDialog.close()}><X size={20} /></button
    >
  </header>
  {#if editing}<form
      onsubmit={(event) => {
        event.preventDefault();
        void save();
      }}
    >
      <label>名称<input bind:value={editing.name} required /></label><label
        >所在文件夹<select bind:value={editing.folderId}
          ><option value={null}>资料根目录</option
          >{#each app.folders as folder}<option value={folder.id}
              >{folderPath(app.folders, folder.id)}</option
            >{/each}</select
        ></label
      ><label>标签（逗号分隔）<input bind:value={tags} /></label><label
        >备注<textarea bind:value={editing.notes} rows="2"></textarea></label
      >{#if editing.kind === 'text'}<label
          >正文<textarea bind:value={editing.text} rows="9"></textarea></label
        >{/if}<button class="button primary" disabled={!!app.busy}
        >保存到草稿</button
      >
    </form>{/if}
</dialog>

{#if folderDialog}<FolderDialog
    folder={folderDialog.folder}
    parentId={folderId}
    onclose={() => (folderDialog = null)}
    onsave={(id) => {
      folderId = id;
      kind = 'all';
    }}
  />{/if}
{#if caseDialog}<CaseEditor
    material={caseDialog.material}
    {folderId}
    onclose={() => (caseDialog = null)}
  />{/if}

<dialog bind:this={importDialog} class="import-dialog">
  <header class="dialog-header">
    <div>
      <span class="eyebrow">EXCEL IMPORT</span>
      <h2>从表格导入话术</h2>
    </div>
    <button
      class="icon-button"
      aria-label="关闭导入"
      onclick={() => importDialog.close()}><X size={20} /></button
    >
  </header>
  <p class="muted">{source?.name} · 使用第一张工作表的首行作为表头。</p>
  <p class="field-help">
    ID 用于重复导入时更新同一条话术。缺少 ID
    时请先在原表添加稳定唯一值；不会按标题或行号猜测身份。
  </p>
  <label
    >流程话术导入到<select bind:value={importFlowId} aria-label="导入所属流程">
      <option value="">请选择流程（仅导入快捷话术时可留空）</option>
      {#each app.flows as flow}<option value={flow.id}>{flow.name}</option
        >{/each}
    </select></label
  >
  {#if !app.flows.length}<p class="field-help">
      请先在“话术与流程”中新建流程，再导入流程话术。
    </p>{/if}
  <div class="mapping-grid">
    {#each fields as [key, label]}<label
        >{label}<select bind:value={mapping[key]}
          ><option value="">不映射 / 使用标准列名</option
          >{#each source?.rows[0]?.cells || [] as header}<option value={header}
              >{header}</option
            >{/each}</select
        ></label
      >{/each}
  </div>
  <p class="field-help">
    中文与韩语回复分别对应模板中的回复列。分支连接也可导入后在流程编辑器补充。
  </p>
  <a class="text-button" href="/script-template.xlsx" download
    >下载标准 Excel 模板<Download size={14} /></a
  >
  <div class="button-row">
    <button
      class="button secondary"
      disabled={!!app.busy}
      onclick={() => parseImport(false)}>预览并校验</button
    ><button
      class="button primary"
      disabled={!!app.busy ||
        !preview.length ||
        previewSignature !== JSON.stringify({ mapping, importFlowId })}
      onclick={() => parseImport(true)}
      >确认导入 {preview.length || ''} 条</button
    >
  </div>
  {#if preview.length}<div class="import-preview">
      <p><Check size={16} />校验通过，共 {preview.length} 条，预览前 20 条</p>
      <table>
        <thead><tr><th>ID</th><th>题目</th><th>语言</th></tr></thead><tbody
          >{#each preview.slice(0, 20) as node}<tr
              ><td>{node.id}</td><td>{node.title}</td><td
                >{Object.keys(node.replies).map(languageName).join('、')}</td
              ></tr
            >{/each}</tbody
        >
      </table>
    </div>{/if}
</dialog>
