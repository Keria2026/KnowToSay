# 开发、构建与验证

## 工具与项目结构

- Bun 1.4.3，按 `bun.lock` 安装前端依赖。
- Rust 1.99.1，包含 rustfmt 和 clippy，版本固定在 `rust-toolchain.toml`。
- Windows 桌面开发需要 Microsoft C++ Build Tools、Windows SDK 和 WebView2。
- Linux 的 Tauri 编译需要 GTK3、WebKitGTK 4.1、相关开发库及图形会话；纯数据与同步测试无需图形依赖。

前端在 `apps/web/`，Rust 本地业务、HTTP 契约、Tauri 入口和局域网同步在 `apps/api/`。Rust 为契约事实来源，`bun run contract` 重新生成 OpenAPI 和 TypeScript 类型。不得手工修改生成类型。

新增依赖的职责：Tauri 插件提供文件选择、剪贴板、全局快捷键、单实例和文件定位；Svelte Flow 用于流程画布；Calamine 用于 Excel 读取；SQLx 用于 SQLite；Axum 用于本地接口和只读局域网发布接口；Playwright 用于真实浏览器验收。

## 启动

在仓库根目录执行：

```sh
bun install --frozen-lockfile
bun run desktop
```

云环境在工作区安装工具时，先执行 `source scripts/dev-env.sh`。使用当前已有检出目录；每个云任务已隔离，不需要另建 Git worktree。

无图形环境可分两个终端运行本地接口与前端，用于开发和自动交互测试：

```sh
bun run api
```

```sh
bun run dev
```

浏览器模式用于交互验证，不能提供原生托盘、系统置顶、全局快捷键或屏幕边缘检测。首次没有资料时可以明确选择添加示例流程；示例不是正式销售内容。

Rust 开发接口默认只监听本机 TCP 4174，前端通过 Vite 代理访问。数据目录默认 `.local/development/`，可通过 `--data-dir` 或 `KNOWTOSAY_DATA_DIR` 指定。桌面程序使用操作系统应用数据目录，禁止将客户资料写进仓库。

## 局域网

管理员创建工作组、编辑草稿并发布后，业务员才可首次加入。发现使用 UDP 51877，已发布资料下载使用 TCP 51876。首次使用应允许 Windows 防火墙的专用网络访问。

自动发现不可用时，填写管理员 IPv4 与端口。连接地址限制为局域网或本机 IPv4 HTTP 地址，支持端口，例如 `192.168.1.10:51876`。每 20 秒检查已发布版本，数据完整后在事务中切换；下载失败保留已验证旧版本与续传文件。仅发布清单和当前素材下载暴露给局域网，编辑与备份接口不对局域网开放。

资料包为含 `manifest.json` 与 `blobs/` 的文件夹；管理员备份为含 `backup.json` 与 `blobs/` 的文件夹。不要混用。恢复原工作组前应关闭旧管理员电脑上的客户端；不支持两个管理员同时发布同一工作组。

业务员可在“工作组与备份”单独导出和恢复个人分支；个人备份不包含统一知识库，不会随管理员备份自动上传。

## Excel 与素材

- 下载客户端内的标准 Excel 模板，或使用 `apps/web/static/script-template.xlsx`；该文件由 `scripts/generate-excel-template.py` 生成。
- 导入已有 Excel 时映射字段。稳定 `id` 与 `title` 必需，同一 ID 重复导入更新草稿而不是创建重复话术。
- 多语言列为 `reply:ko`、`reply:zh-CN` 等；变量、分支和素材 ID 列可用 JSON 数组，也可导入后在编辑器补充。
- `kind` 列区分 `flow`（流程话术）和 `quick`（快捷话术），也接受中文类型名。未映射或留空时按流程话术导入；快捷话术不能设置入口、分支或作为流程连接目标。标准模板包含此列，已有资料不按分支数量自动重新分类。
- Excel 当前支持 XLSX、XLS、XLSB、ODS；单个工作簿最多索引 100,000 个非空行，发布清单最多 64MB。实际业务文件需预先验证格式与容量。
- 图片与视频按需预览；具体编解码器受 Windows/WebView2 支持范围影响。无法预览时导出原文件使用本机软件。
- 素材内容按 SHA-256 去重保存。被移除素材不能通过当前界面或发布下载接口继续访问；旧文件为回滚和备份保留，自动空间回收策略尚未启用。

## 自动检查

```sh
bun run format:check
bun run check
bun test apps/web/src
bun run build
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
bun run contract
```

浏览器端到端测试使用独立的随机测试目录，接口端口 4175、开发前端 1421、生产预览 1423，不接触日常资料库：

```sh
bun x playwright install chromium
bun run test:e2e
```

云环境已有 Chromium 时可以指定 `PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH=/usr/bin/chromium`。`test:e2e` 先生成生产静态构建，再执行建组、变量校验与复制、分支汇合、悬浮布局切换、Excel 映射导入、独立快捷话术编辑、单元格搜索、发布和重启持久化。独立窗口使用生产预览根路由验证快捷回复表、直接复制、全库搜索和返回工作台，防止打包路径再次出现 404。截图与失败 trace 保存在忽略的 `test-results/`。

后端测试通过真实 SQLite 和回环 HTTP 服务覆盖草稿隔离、角色边界、流程与快捷话术类型校验、个人分支更新保护、素材删除、Range 续传、SHA-256 失败不切换、备份恢复、资料包和 Excel 超出预览范围的搜索。屏幕边缘逻辑另测禁用边缘、角落及负坐标显示器；根路由解析覆盖开发及两种 Tauri 协议。这些不能证明 Windows 桌面消息和真实鼠标唤出已验证。

## Windows 安装包

```sh
bun run desktop:build
```

本地默认构建输出位于 `target/release/bundle/nsis/`。仓库工作流对 `dev`、`master` 推送及 PR 运行检查，并执行 `bun run desktop:build --target x86_64-pc-windows-msvc --bundles nsis --ci`，生成 Windows x64 NSIS 安装包。CI 输出位于 `target/x86_64-pc-windows-msvc/release/bundle/nsis/`，产物名为 `KnowToSay-Windows-installer`，保留 14 天；缺少 `.exe` 时任务必须失败。提交工作流不代表该次远端运行已经成功。

Git 使用 `.gitattributes` 将文本文件统一检出为 LF，Prettier 同样检查 LF，避免 Windows 的 `core.autocrlf=true` 使所有前端文件在格式检查阶段失败。不得通过跳过格式检查来解决换行差异。

Windows CI 在安装包构建前安装 Chromium 并执行上述浏览器回归；生产静态页面路由和快捷回复复制失败时不生成安装包。

## 交付前仍需目标环境验收

1. Windows 10/11 安装、启动、退出、WebView2 可用性和离线重启。
2. 托盘最小化、关闭到托盘、退出、快捷键冲突与恢复、四个屏幕边缘、多显示器/DPI、全屏及拖拽抑制。
3. 独立搜索栏复制后收起和焦点恢复、与 LDCLOUD / KakaoTalk 的实际文本粘贴及素材上传。
4. 两台真实电脑的局域网发现、防火墙、管理员离线，以及 20–30GB 素材传输、磁盘不足、断网和断电恢复。

当前自动验证不代替上述人工验收。安装包构建、安装成功和实际业务流程验收分别记录，不统称为已上线或已完整验收。
