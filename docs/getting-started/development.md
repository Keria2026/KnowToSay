# 开发、构建与验证

## 工具与运行方式

- Bun 1.4.3，按根目录 `bun.lock` 安装前端依赖。
- Rust 1.99.1，包含 rustfmt 和 clippy，版本固定在 `rust-toolchain.toml`。
- Windows 桌面开发需要 Microsoft C++ Build Tools、Windows SDK 和 WebView2。
- Linux 原生 Tauri 编译需要 GTK3、WebKitGTK 4.1 等开发库，运行桌面界面还需要图形会话；纯数据与浏览器测试不要求图形桌面。
- 每台客户端使用本地 SQLite 和素材文件，不需要 PostgreSQL、Docker Compose 或在线账号。

前端位于 `apps/web/`，SvelteKit 生成静态资源供 Tauri 加载。本地业务、SQLite、文件处理和同步位于 `apps/api/`。桌面通过 Tauri IPC 调用 Rust；浏览器开发模式通过 Vite 代理访问 Axum 本机接口，不另设生产公网服务端。

依赖职责：Tauri 插件负责文件选择、剪贴板、快捷键和单实例等原生能力；Svelte Flow 负责流程画布；Calamine 读取 Excel；SQLx 访问 SQLite；Axum 提供本机开发接口及管理员局域网发布接口；Playwright 验证真实浏览器交互。

## 安装与启动

从仓库根目录执行：

```sh
bun install --frozen-lockfile
bun run desktop
```

云环境使用工作区工具时，先执行 `source scripts/dev-env.sh`。使用已有检出目录；每个云任务已隔离，除非用户明确要求，不另建 Git worktree。

无图形环境在两个终端运行：

```sh
bun run api
```

```sh
bun run dev
```

开发 API 默认监听 `127.0.0.1:4174`，Vite 监听 `127.0.0.1:1420` 并代理 `/api`。通过本机请求确认首页返回 HTTP 200、`/api/v1/snapshot` 返回有效快照，而非只检查端口或进程。

不需要局域网发现与后台同步时，可用以下命令替代 `bun run api`：

```sh
cargo run --locked -p knowtosay --bin knowtosay-api -- --data-dir .local/development --port 4174 --no-lan
```

开发数据默认位于 `.local/development/`，可用 `--data-dir` 或 `KNOWTOSAY_DATA_DIR` 指定。原生桌面使用操作系统应用数据目录。数据目录包含 `workspace.sqlite`、`blobs/`、`staging/` 等运行文件；正式客户资料不能写入或提交仓库。

首次运行明确创建或加入工作组；没有话术时可以选择添加示例流程，示例不能当作正式销售内容。浏览器可验证页面、全屏与复制，不能证明托盘、置顶、原生全屏、全局快捷键或屏幕边缘检测已经可用。

## 页面、窗口与流程

完整窗口有销售工作台、话术与流程、知识库、设置四个页面。置顶模式使用 `CompactWorkbench`，只提供已有流程、快捷话术、知识库的查找与复制/导出，不暴露编辑和管理。完整页面保留在隐藏且 inert 的容器中，切回时保留编辑草稿；销售工具通过 `focusScript` 更新使用中的步骤，不触发离开编辑器或丢弃草稿。窗口恢复保留进入悬浮模式前的尺寸、位置和最大化状态。

`ScriptResult`、`MaterialResult` 在全局搜索、独立搜索及悬浮工具中提供直接复制或导出；话术有变量时先打开详情。`ReplyDialog` 展示完整双语内容与复制预览。`SearchScopePicker` 默认全局，可限定某条流程、快捷分类或知识库分类；知识库分类在服务端先过滤，再限制搜索数量，避免被其他分类的命中挤掉。独立搜索通过快捷键唤出默认全局范围，边缘唤出默认快捷话术范围；同一范围选择器切换时保留关键词，不另设回复表模式。`SearchResultsModel` 统一工作台与独立窗口的筛选、分页、异步素材搜索和过期请求结果丢弃；无关键词仅在快捷话术范围直接浏览，不扫描知识库。

工作台和编辑页通过 `ScriptNavigation` 与 `ScriptGroupTabs` 共用“话术类型 → 流程或快捷分类”导航。流程话术按 `flowId` 归属，连接只能在同一流程内；快捷话术不属于流程，不能有入口、分支或成为分支目标。类型在创建入口确定，空流程首个新建步骤自动为入口，不显示类型切换或入口复选框。

流程与快捷分类可拖动、用 Alt + 左右方向键或排序对话框调整顺序。本机 WebView localStorage 按工作组分别存储：

| 偏好         | 键                                         |
| ------------ | ------------------------------------------ |
| 流程顺序     | `knowtosay.flow-order.<groupId>`           |
| 快捷分类顺序 | `knowtosay.quick-category-order.<groupId>` |

顺序在工作台、编辑页和悬浮窗共用，不随主库发布；新项目追加在末尾，失效标识或损坏偏好不会替代实际资料。

工作台完整图包括所选流程的全部步骤，含未连接节点；不按连通分量过滤。编辑器保留手动坐标，只有明确点击自动排列才重排当前流程。分支条件显示在来源节点内，每个条件有独立连接点。

工作台与编辑页均可进入全屏流程查看。`FlowFullscreen.svelte` 使用浏览器 Fullscreen API 或 Tauri 原生窗口全屏，右侧显示节点详情；查看选择与工作台或编辑器当前步骤分离。退出、Esc 或 F11 恢复原窗口；只有明确选择继续工作台或返回编辑才定位原页面。编辑页查看当前未保存记录，退出不丢失草稿。原生权限声明在 `apps/api/capabilities/main.json`，请求失败有窗口内全屏退路。

详细角色、复制和窗口交互见[桌面页面与操作流程](../architecture/desktop-interactions.md)。

## SQLite、契约与资料模型

数据库由 `AppState::open` 打开并运行 `apps/api/migrations/` 的 SQLx 迁移，启用 WAL、Full synchronous 和外键。业务查询使用 SQLx 运行时查询；当前没有 `.sqlx/` 离线元数据目录。不要直接改写已执行迁移或复制正在写入的数据库作为备份。

Rust 是契约事实来源，运行：

```sh
bun run contract
git diff --exit-code -- apps/web/src/lib/generated/api.ts apps/web/static/openapi.json
```

该命令更新生成文件；契约变更时应审阅并提交生成差异，无契约变更时差异检查应为空。不得手工修改生成的 TypeScript 或 OpenAPI。

| 本地接口                                     | 职责                                       |
| -------------------------------------------- | ------------------------------------------ |
| `GET /api/v1/snapshot`                       | 当前角色有效资料与主库草稿状态             |
| `PUT /api/v1/scripts`                        | 事务保存所有实际变化的话术、坐标与连接     |
| `PUT /api/v1/flows/{id}`                     | 新建或重命名流程                           |
| `POST /api/v1/quick-categories`              | 按角色创建主库草稿或个人快捷分类           |
| `PUT/DELETE /api/v1/quick-categories/{name}` | 事务重命名或删除快捷分类并更新话术归类     |
| `POST /api/v1/backup/import`                 | 将其他工作组备份的草稿追加到当前管理员主库 |

完整契约见 `apps/web/static/openapi.json`，路由组装见 `apps/api/src/http.rs`。桌面 IPC 复用同一本地业务；局域网不开放这些编辑接口。

管理员工作台读取主库草稿，保存后不自动发布。业务员有效视图为个人覆盖优先，其余使用当前已安装的官方版本；个人修改保留所需官方步骤和流程依赖。管理员删除这些依赖后可以保留话术上下文，但已删除素材不可因个人引用继续下载。

流程记录按草稿、官方、个人及个人依赖分表存储。快捷分类同样独立保存元数据，空分类随发布、资料包、管理员备份和个人备份保留。`Snapshot.manageableQuickCategories` 区分可管理的草稿或个人分类与只读官方分类。删除分类将话术移到“未分类”，前端同时维护未保存编辑与本机顺序。知识库分类与快捷分类是独立模型。

当前资料格式为 2，同一工作组客户端均应使用支持多流程的版本。单画布数据库迁移与旧格式资料包、备份导入会将原画布归入“原有流程”，保留步骤 ID、连接、坐标和个人内容，不按分类或标题猜测拆分。这些升级路径仍保护已有用户资料，不能作为废弃实现删除。

## 局域网与资料交换

管理员创建工作组并发布后，业务员才能首次加入。UDP 51877 用于发现，TCP 51876 用于已发布清单和素材下载。Windows 首次使用需允许专用网络防火墙访问。

发现不可用时填写管理员局域网 IPv4 和端口，例如 `192.168.1.10:51876`；客户端仅接受局域网或回环 IPv4 HTTP 地址，不跟随重定向。每 20 秒检查更新。资料先进入暂存目录，检查所需空间、SHA-256 及完整性后事务切换；失败保留当前版本与续传数据。编辑、备份和恢复接口不向局域网开放。

| 资料类型     | 内容入口                    | 导入或恢复语义                                           |
| ------------ | --------------------------- | -------------------------------------------------------- |
| 已发布资料包 | `manifest.json` 与 `blobs/` | 离线分发管理员发布版本，保留业务员个人内容               |
| 管理员备份   | `backup.json` 与 `blobs/`   | 恢复原工作组，或由管理员明确选择跨组追加导入             |
| 个人备份     | `personal.json`             | 仅同工作组业务员恢复个人内容及必要依赖，不包含知识库素材 |

恢复原工作组前关闭旧管理员，不能运行两个管理员同时发布。同组恢复会恢复主库并创建新发布版本；个人恢复替换当前个人内容，建议先备份。管理员备份不包含其他电脑的个人分支。

跨工作组备份导入在校验后为话术、流程、分支和素材引用重新映射标识，保留排版，同名流程自动加后缀；内容追加到当前主库草稿，不改变身份、当前发布版本或已有资料。数据库写入为同一事务；素材按校验值复用，空间检查只计待复制文件。导入完成后检查并另行发布。

## Excel 与素材

- 标准模板为 `apps/web/static/script-template.xlsx`，由 `scripts/generate-excel-template.py` 生成，也可在客户端内下载。
- 导入流程话术前新建流程，并在导入对话框选择归属；快捷话术不属于流程。
- 预览并映射已有 Excel 字段；稳定 `id` 与 `title` 必需。同 ID 重复导入更新草稿，不以行号或标题猜测身份。
- 多语言列如 `reply:ko`、`reply:zh-CN`；变量、分支和素材 ID 列可用 JSON 数组，也可导入后补充。
- `kind` 接受 `flow`、`quick` 或对应中文名称，未映射或留空按流程话术导入；不能根据有无分支自动改变类型。
- 支持 XLSX、XLS、XLSB、ODS，单个工作簿最多索引 100,000 个非空行，发布清单最多 64MB。实际业务文件仍需验证容量与格式。
- 素材导入后在本地保存，移走源文件不影响已入库资料；内容按 SHA-256 去重。图片和视频按需预览，实际编解码器由 Windows/WebView2 决定。
- 管理员删除素材并发布，业务员成功同步后才移除可用入口；个人话术中的失效引用保留并提示。历史文件保留用于备份或回滚，自动空间清理尚未实现。

`SheetPreview` 使用 `/materials/{id}/rows` 的 `sheet`、`q`、`offset` 参数获取分页。`SheetPage` 包含全部工作表名、所选表的列数、匹配总行数及最多 150 行数据；界面保留空格位和原行号，提供工作表选择、内部横向滚动和列组切换。新读取 Excel 时补齐范围起始列之前的空列，避免 C3 内容误显示为 A3。测试文件 `apps/web/tests/fixtures/sales-grid.xlsx` 覆盖非 A 列起点、多工作表、AA 列与超过一页数据。

桌面设置支持录入组合键和手动修改，快捷键冲突时保留原注册，不能将浏览器预览当成系统全局快捷键验证。独立搜索窗可调整大小；展开结果只触发一次尺寸调整并保留用户宽度，窗口尺寸限制在当前显示器内。

## 自动检查

仓库根目录运行：

```sh
bun run format:check
bun run check
bun test apps/web/src
bun run build
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

契约更新另按上文生成并核对。浏览器回归：

```sh
bunx playwright install chromium
bun run test:e2e
```

云环境已有系统 Chromium 时，可替代为：

```sh
PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH=/usr/bin/chromium bun run test:e2e
```

Playwright 自动构建生产静态页面，并管理 API 4175、开发前端 1421、生产预览 1423，使用 `.local/e2e-<随机标识>`，不接触日常资料库。截图与失败 trace 位于忽略的 `test-results/`。

| 检查           | 主要覆盖                                                                                                                            |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| 前端单元测试   | 复制变量与缺失语言校验、Excel 定位、分支子树和汇合排布、循环及 2000 步排列、本机标签顺序                                            |
| Rust 测试      | 真实 SQLite、角色与草稿隔离、发布与个人依赖、事务失败回滚、素材删除与 SHA-256、Range 续传、资料包与三种备份操作、多流程和空分类迁移 |
| 浏览器回归     | 建组、导航与复制、图表拖动连线、未保存内容保护、流程和快捷分类排序、窄悬浮窗各页、知识库与 Excel、跨组导入、生产静态搜索窗          |
| 品牌与全屏回归 | 应用内 Logo 引用、导航当前状态、浏览器实际全屏、右侧节点详情、退出后步骤与编辑保留                                                  |
| Windows 打包   | CI 的 Windows MSVC / NSIS 构建；构建成功不等同安装与原生行为通过                                                                    |

具体通过记录以当次命令结果为准，不把零测试、跳过或未运行当通过。

## Logo 与 Windows 安装包

应用内品牌统一引用 `apps/web/static/icon.svg`，`apps/api/icons/logo-apple.svg` 保存同版矢量母版。桌面 `icon.png`、不同尺寸 PNG 与 ICO 必须使用同一设计。更新时用 Tauri 工具生成到临时目录，不能只替换网页图标：

```sh
bunx tauri icon apps/web/static/icon.svg --output .local/generated-icons
```

将生成的 `32x32.png`、`128x128.png`、`128x128@2x.png`、`icon.png`、`icon.ico` 替换到 `apps/api/icons/`，同时更新独立快捷方式资源 `brand-v2.ico`。未来再更换品牌时需为独立资源换新文件名，并同步配置和安装钩子，避免复用 Explorer 缓存。PNG 使用 Tauri 生成的 RGBA 格式，不保留旧版 `icon.png` 或仅改后缀。

`tauri.conf.json` 定义打包图标、安装器和卸载器图标及独立资源；`installer-hooks.nsh` 在升级时刷新指向本应用的已有桌面和开始菜单快捷方式，通知 Explorer 更新图标缓存。此实现仍需用真实旧版升级验收，不代表已验证所有 Windows 缓存行为。

在具备 Windows 工具链的环境中运行：

```sh
bun run desktop:build
```

本地默认输出为 `target/release/bundle/nsis/`。`.github/workflows/ci.yml` 对 `dev`、`master` 推送及 PR 运行格式、类型、单元/集成、契约和浏览器检查，再执行：

```sh
bun run desktop:build --target x86_64-pc-windows-msvc --bundles nsis --ci
```

CI 输出位于 `target/x86_64-pc-windows-msvc/release/bundle/nsis/`，安装包产物名 `KnowToSay-Windows-installer`，保留 14 天，缺少 `.exe` 时任务失败。浏览器失败时保存日志、截图和 trace 至 `KnowToSay-UI-test-diagnostics`，保留 7 天。推送工作流不能代替检查该次远端结果。

Git 与 Prettier 统一 LF；不能用跳过格式检查解决 Windows 换行差异。Windows 浏览器测试允许首次冷启动时间，其余交互保持正常断言。

## 仍需目标环境验收

1. Windows 10/11 首次安装、旧版升级、桌面/开始菜单/窗口图标、WebView2 可用性、退出及离线重启。
2. 原生流程全屏、Esc/F11 退出、原窗口状态恢复、窄悬浮窗与多显示器/DPI。
3. 最小化或关闭后托盘常驻、明确退出、快捷键冲突与恢复、四边检测、全屏与拖拽抑制。
4. 独立搜索窗复制后收起和焦点恢复、真实 LDCLOUD / KakaoTalk 文本粘贴与素材手动上传。
5. 两台真机的发现、防火墙、管理员离线，以及 20–30GB 素材传输、磁盘不足、断网、断电恢复与实际媒体格式。

自动检查、安装包生成、安装成功和业务验收是不同结果，分别记录。

本次浏览器回归覆盖只读悬浮窗与完整编辑器草稿共存、直接复制和变量校验、搜索范围与分组、真实 Excel 的多工作表/原列/分页，以及 340px、800px、1440px 的四页布局。原生快捷键注册、多显示器 DPI 和焦点恢复仍需 Windows 真机验收。
