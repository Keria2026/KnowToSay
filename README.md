# KnowToSay

面向多语言销售的话术与知识库桌面系统，目标平台为 Windows 10 及以上版本。

首版采用 Tauri、Svelte 5、Rust / SQLx 和 SQLite，支持本地话术、流程编辑、知识库、管理员发布与局域网增量同步，以及业务员独立编辑个人话术分支。

已实现多流程与快捷分类导航、流程编辑和全屏查看、右侧节点详情、知识库、完整工作台与精简悬浮窗口，以及托盘常驻和独立快捷搜索栏。发布、同步、工作组和备份统一在设置页，支持导入其他工作组备份为本地草稿。Windows 的全局快捷键默认 `Ctrl + Alt + Space`；收起工作台后默认在右边缘停留 400ms 唤出快捷话术回复表，四边与停留时间可配置。

当前为开发版本：本地数据、同步核心、浏览器交互可自动验证，CI 配置为构建 Windows 安装包；实际 Windows 安装升级、图标刷新、原生全屏、托盘与屏幕边缘行为、LDCLOUD / KakaoTalk 粘贴和 20–30GB 真机传输仍需目标环境验收。

- [安装、启动与验证](docs/getting-started/development.md)
- [桌面页面与操作流程](docs/architecture/desktop-interactions.md)
- [文档索引](docs/README.md)
- [首版范围](docs/architecture/v1-scope.md)
- [开发规范](AGENTS.md)
- [设计指南](DESIGN.md)

SQLite 桌面端例外已获用户批准，见 [架构决策](docs/adr/0001-offline-desktop-storage.md)。
