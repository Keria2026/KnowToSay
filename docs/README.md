# 文档索引

## 需求与架构

- [开发环境、运行命令与 Windows 验收](getting-started/development.md)
- [首版范围与验收场景](architecture/v1-scope.md)
- [桌面页面与操作流程](architecture/desktop-interactions.md)
- [领域术语](architecture/domain-glossary.md)
- [已批准：离线桌面端与 SQLite 存储](adr/0001-offline-desktop-storage.md)

## 核心指南

- [AGENTS：工程规范](../AGENTS.md)
- [DESIGN：视觉设计指南](../DESIGN.md)

首版范围记录已确认边界；页面与操作流程描述当前实现；领域术语区分主库草稿、个人内容、发布、备份与导入；开发指南提供运行命令、格式升级和检查方法。工程规范已按 Tauri / SQLite 桌面架构统一，历史存储取舍保留在 ADR。

自动检查、CI 安装包构建和真实 Windows / LDCLOUD 验收分别记录；代码或文档存在不代表目标场景已经验收通过。
