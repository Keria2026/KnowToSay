# KnowToSay

面向多语言销售的话术与知识库桌面系统，目标平台为 Windows 10 及以上版本。

首版采用 Tauri、Svelte 5、Rust / SQLx 和 SQLite，支持本地话术、流程编辑、知识库、管理员发布与局域网增量同步，以及业务员独立编辑个人话术分支。

已实现完整工作台、精简悬浮窗口、托盘常驻和独立快捷搜索栏。Windows 的全局快捷键默认 `Ctrl + Alt + Space`；收起工作台后默认在右边缘停留 400ms 唤出搜索，四边与停留时间可配置。

当前为开发版本：本地数据、同步核心、浏览器交互及 Windows 目标编译可自动验证；实际 Windows 安装、托盘与屏幕边缘行为、LDCLOUD / KakaoTalk 粘贴和 20–30GB 真机传输仍需目标环境验收。

- [安装、启动与验证](docs/getting-started/development.md)

- [文档索引](docs/README.md)
- [首版范围](docs/architecture/v1-scope.md)
- [开发规范](AGENTS.md)
- [设计指南](DESIGN.md)

SQLite 桌面端例外已获用户批准，见 [架构决策](docs/adr/0001-offline-desktop-storage.md)。
