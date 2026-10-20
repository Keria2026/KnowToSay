# AGENTS.md — 工程规范与当前架构

本文适用于所有开发人员和 AI 编码代理。先阅读本文件、[首版范围](docs/architecture/v1-scope.md)、[领域术语](docs/architecture/domain-glossary.md)和[桌面存储决策](docs/adr/0001-offline-desktop-storage.md)。界面遵循 [DESIGN.md](DESIGN.md)，桌面适配规则见[页面与操作流程](docs/architecture/desktop-interactions.md)。

## 1. 开发原则

- 做长期正确的事；优先正确性、可维护性、简洁性、一致性和可测试性。
- 遵循 KISS、DRY、YAGNI，复用现有模块，按业务边界组织代码。
- 结构性问题通过必要重构解决，替换实现后清除不再使用的旧实现。
- 同一业务能力使用统一入口与状态模型；范围、分类和触发方式通过参数表达，不另建重复模式、页面或筛选状态。重构须删除被替代的组件、分支、样式、原生命令和文档，并验证原有操作。
- 不为未确认需求引入抽象、兼容层、框架或目录。
- 保留已发布数据所需的迁移和格式升级；不能把仍用于读取用户资料的代码当作无用历史删除。
- 代码、数据库、API 契约、生成类型和文档保持一致，不保留互相矛盾的规范。
- 不覆盖用户改动，不在代码、日志、文档或提交中写入密钥与客户资料。

## 2. 固定技术栈与运行边界

| 层次     | 当前技术与职责                                                                    |
| -------- | --------------------------------------------------------------------------------- |
| 目标平台 | Windows 10+ 的 Tauri 2 桌面客户端                                                 |
| 前端     | Svelte 5 Runes、SvelteKit、TypeScript Strict、Vite；生产构建为静态资源            |
| 前端工具 | Bun，版本见 `package.json`，依赖以 `bun.lock` 为准                                |
| 本地业务 | Rust Stable、Tokio、SQLx；版本见 `rust-toolchain.toml`                            |
| 数据     | 每台客户端独立 SQLite，素材文件保存在应用数据目录                                 |
| 本地调用 | 桌面通过 Tauri IPC 调用 Rust；浏览器开发模式通过 Vite 代理调用本机 Axum HTTP 接口 |
| 局域网   | 管理员内置 Axum 只读发布服务和 UDP 发现；业务员下载已发布资料                     |
| 交付     | GitHub Actions 验证并构建 Windows x64 NSIS 安装包                                 |

首版没有独立公网服务端，不需要 PostgreSQL、Docker Compose、在线账号或数据库凭据。将来独立服务端的数据库和部署方式尚未决定，不能据此引入新依赖。未经明确批准不得更换上述技术栈。

## 3. 目录与模块

使用现有检出目录；云任务本身已隔离，除非用户明确要求，不创建额外 Git worktree。只为实际功能创建目录，不强制补齐空的目标结构。

```text
apps/web/
  src/routes/              SvelteKit 页面与布局组合
  src/lib/features/
    scripts/               流程、快捷话术、编辑器与只读全屏查看
    knowledge/             知识库、素材与导入
    workspace/             工作组、共享状态、搜索、设置与本地 API 适配
  src/lib/generated/       从 Rust OpenAPI 生成的 TypeScript 类型
  src/lib/utils/           复用的纯函数
  src/lib/types.ts          生成契约的类型导出
  src/app.css              共享样式与设计变量
  static/                  静态资源、图标、Excel 模板与 OpenAPI
  tests/                   Playwright 浏览器回归
apps/api/
  src/                     Rust 本地业务与桌面入口
  migrations/              SQLx SQLite 版本迁移
  tests/                   SQLite、HTTP、同步与备份集成测试
  icons/                   桌面 PNG/ICO 资源
  capabilities/            Tauri 权限声明
  tauri.conf.json          窗口、安全、构建与安装器配置
  installer-hooks.nsh      Windows 升级快捷方式图标刷新
scripts/                   已有开发辅助与模板生成脚本
docs/                      正式需求、架构、开发与决策文档
.github/workflows/         验证与 Windows 打包
```

前端字体通过依赖打包，不依赖在线字体。不要为代码分层形式创建无实际职责的转发组件或空目录。

### 前端规则

- `routes/` 负责页面组合、加载和交互入口，业务能力放入已有 Feature。
- 使用 Svelte 5 Runes 和 TypeScript Strict；已有共享状态位于 `workspace/store.svelte.ts`，不另建重复状态体系。
- 桌面与浏览器复用业务组件，通过 `workspace/api.ts` 适配调用方式；不能在浏览器代码访问 SQLite 或暴露系统私有能力。
- 工作台与编辑器复用话术类型、流程和快捷分类导航。完整窗口承载编辑与管理；置顶悬浮窗仅用于查找、选择、预览与复制，复用业务状态和复制、预览组件，切换保留未保存编辑。
- 流程查看和流程编辑职责分离。全屏查看不能隐式修改布局、连接、工作台步骤或编辑草稿。
- 使用 `app.css` 的色彩、字体、间距和圆角变量；正文 17px / 400，强调 600，单一蓝色交互信号。界面卡片、按钮和文字不增加装饰阴影或渐变。
- 左侧导航当前项使用浅蓝底、蓝色强调和 `aria-current`；其他项浅灰。应用内 Logo 引用 `static/icon.svg`，桌面图标必须同步生成。
- 支持窄窗口、键盘操作和清晰焦点，主操作点击区域至少 44px。

### Rust 模块职责

| 文件                                        | 职责                                                        |
| ------------------------------------------- | ----------------------------------------------------------- |
| `main.rs`                                   | 浏览器开发用本机 HTTP 服务入口                              |
| `desktop.rs`                                | Tauri 入口、本地命令和原生文件操作                          |
| `desktop_support.rs`、`desktop_behavior.rs` | 托盘、搜索窗、快捷键、屏幕边缘与可测试的窗口行为规则        |
| `http.rs`                                   | 本地与局域网路由、请求解析和用例调用                        |
| `db.rs`                                     | AppState、SQLite 读写、草稿、个人覆盖、流程与分类、发布事务 |
| `sync.rs`                                   | 发现、下载、校验、续传与发布版本切换                        |
| `backup.rs`、`personal.rs`                  | 资料包、管理员备份、跨组资料导入与个人备份                  |
| `folders.rs`                              | 文件夹树、目录操作与旧分类资料升级                         |
| `files.rs`、`search.rs`                     | 素材、Excel 读取与索引搜索                                  |
| `models.rs`、`contract.rs`                  | 领域 DTO 与 OpenAPI 契约                                    |
| `error.rs`                                  | 错误码及统一响应                                            |

在当前模块边界内保持职责清楚；不要求迁移到不存在的 `modules/`、Handler/Service/Repository 目录。业务函数不要依赖前端 UI 状态，通用能力只有存在真实复用需求时才提取。

使用 Rust 官方命名、rustfmt、clippy 和明确的错误传播；正常可恢复路径不随意使用 `unwrap()` 或 `expect()`。异步代码避免长时间阻塞，不暴露数据库错误或堆栈给界面。

## 4. 数据与同步不变量

- 所有数据库操作由 Rust / SQLx 负责；SQL 值使用参数绑定，动态表名只能从已校验的内部白名单选择。
- SQLite 数据目录包含 `workspace.sqlite`、`blobs/` 和 `staging/` 等运行文件；正式客户资料位于操作系统应用数据目录。开发临时数据仅放在忽略的 `.local/`，不能提交。
- 管理员主库草稿、管理员已发布资料和业务员个人内容分别保存。管理员工作台读取草稿；业务员使用已发布资料并优先应用个人覆盖。
- “保存”是持久化本地修改，“发布”才使业务员可同步；界面未保存草稿与已保存但未发布的主库草稿是不同状态。
- 一条流程有稳定 ID 和独立名称；流程步骤按 `flowId` 归属，连接只能在同一流程内，不能指向快捷话术。空流程和空分类也属于有效资料。
- 知识库使用带稳定 ID 的文件夹树，改名和移动不改变引用；案例包含文案及有顺序的图片引用，发布、同步、备份及跨组导入必须同时保留文件夹关系和原图。旧分类字段仅用于已发布资料升级。
- 个人修改及其流程依赖不能被同步静默覆盖；恢复管理员版本是明确操作。个人依赖保留不意味着已删除的素材仍可用。
- 下载先暂存、校验 SHA-256 和所需磁盘空间，数据与素材全部就绪后事务切换版本；失败继续使用旧版本并保留续传数据。
- 管理员恢复、跨组导入和个人恢复有不同的数据边界，遵循开发指南，不能混用文件格式或整体覆盖业务员数据库。

### 迁移与类型

- 新 Schema 变更新增 `apps/api/migrations/` 迁移，不改写已共享或执行的历史迁移，不手动绕过迁移改库。
- 数据库表与字段用 `snake_case`；JSON 字段用 `camelCase`；标识不可从标题、行号或列表位置推测。
- 时间按 UTC 语义存储，使用 SQLite 支持的类型与当前 DTO 格式，不使用 `TIMESTAMPTZ` 等其他数据库专有类型。
- 当前实现通过 `sqlx::query` / `query_scalar` 等运行时查询访问 SQLite，没有 `.sqlx/` 离线查询元数据要求。以后引入编译期查询宏时再配置相应校验，不创建空元数据目录。

## 5. API 与生成契约

Rust DTO 是契约的唯一事实来源：`models.rs` / `contract.rs` → OpenAPI → TypeScript。运行 `bun run contract` 生成 `apps/web/static/openapi.json` 和 `apps/web/src/lib/generated/api.ts`，不手工修改生成文件。

本地 HTTP 路由使用 `/api/v1`，包括 `/snapshot`、`/scripts`、`/flows/{id}`、`/quick-categories`、`/materials`、`/publish`、`/backup/import` 等。桌面通过 IPC 复用相同业务；局域网只暴露发布清单与当前已发布素材下载，不开放编辑、备份或恢复接口。新增原生能力必须同时检查 Tauri 权限和 CSP。

错误响应字段为 `code`、`message`、`requestId`、`details`；错误码保持稳定，说明使用简体中文，不把本地身份分工描述为在线认证或防复制能力。

## 6. 依赖、文档与命名

- 项目前端只用 Bun，后端只用 Cargo；提交 `bun.lock` 和 `Cargo.lock`，不提交 npm、pnpm 或 yarn 锁文件。
- 优先复用依赖。增加依赖需说明必要性，不为简单功能引入大型框架或重复能力。
- 组件、类型用 PascalCase，TS 变量和函数用 camelCase，Rust 模块、函数及数据库标识用 snake_case，环境变量用 SCREAMING_SNAKE_CASE。
- 正式技术文档集中于 `docs/`，根目录保留 README、AGENTS、DESIGN 和工具必要文件；入口为 [文档索引](docs/README.md)。不新增重复的 CONTEXT、NOTES、SUMMARY 等文档，术语直接维护现有领域术语表。
- 说明使用简体中文；Markdown 文件用 kebab-case 和相对链接。优先更新原文，删除已被取代的描述，保留架构决策的历史理由和真实未验收事项。
- UI 实现按 DESIGN 的桌面适配规则维护；通用设计参考不当作已经实现的产品页面或强制增加营销页组件。

## 7. 验证与交付

命令及平台前提以[开发指南](docs/getting-started/development.md)为准。依变更执行相关检查，不声称运行过未执行的测试；纯文档修改验证格式、链接和代码依据，无需重复构建应用。

代码变更至少按影响范围检查：

```sh
bun run format:check
bun run check
bun test apps/web/src
bun run build
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

契约变更还须重新生成并核对 OpenAPI 与 TypeScript；交互变更执行 `bun run test:e2e`。检查必须实际执行目标，零测试、跳过和未运行不能算通过。

GitHub Actions 的 `.github/workflows/ci.yml` 对 `dev`、`master` 推送及 PR 验证，检查成功后构建 NSIS 安装包。不能把推送、CI 构建、安装成功、Windows 原生功能和真机业务验收混为一谈；不宣称未验证的远端结果。

首版交付是桌面安装包，没有 Docker 镜像部署、服务端迁移或上线健康检查流程。安装、升级、离线重启、托盘、图标缓存、原生全屏、多屏 DPI、快捷键及真实局域网需要目标平台验收。

## 8. 工作边界

执行前检查工作区与现有代码，执行后检查差异、生成物、格式、相关测试和文档。只修改任务必要范围，发现与任务无关的问题如实记录，不顺带大规模重构。

以下决策需明确批准：更换框架或数据库、引入微服务/Kubernetes/消息队列/新 ORM/新状态体系、创建新顶层目录、改变通信协议或大规模重组项目。已经批准的 Tauri、SQLite、本机 IPC 与管理员内置局域网服务就是当前架构，不重复要求批准。
