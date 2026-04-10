# 项目开发规范

本文件为 Codex 提供项目上下文与开发约定，所有生成代码、提交信息、架构设计均须遵守以下规则。

---

## Response rules
- 回答我时，先输出一行：AGENTS_OK

## 技术栈

- **桌面框架**：Tauri 2
- **前端框架**：Vue 3（Composition API + `<script setup>`）
- **包管理器**：pnpm（禁止使用 npm / yarn）
- **语言**：JavaScript（前端）、Rust（Tauri 后端命令）

---

## 一、依赖选型原则

1. **优先官方**：优先使用框架或生态官方提供的插件与库。
   - Tauri 功能优先使用 `@tauri-apps/plugin-*` 官方插件，禁止自行封装已有官方实现的能力。
   - Vue 生态优先使用 `vue-router`、`pinia`，禁止引入 `vuex`。
2. **最小依赖**：不引入可用原生 API 或已有依赖能力覆盖的第三方库。
3. **评估后引入**：引入新依赖前须确认其维护状态、包体积影响及 Tauri 2 兼容性。
4. **安装命令**：始终使用 `pnpm add`，禁止使用 `npm install` 或 `yarn add`。

---

## 二、代码风格与注释规范

### 通用

- 缩进：2 空格。
- 字符串：优先单引号；模板字符串场景除外。
- 行尾：无分号（遵循项目 ESLint / Prettier 配置）。
- 所有注释使用**中文**。

### Vue 3

- 组件统一使用 `<script setup lang="ts">` 语法。
- Props 必须使用 `defineProps<{}>()` 泛型形式，禁止 `defineProps([])` 数组形式。
- Emits 必须使用 `defineEmits<{}>()` 泛型形式。
- 组件文件名使用 PascalCase，例如 `UserProfile.vue`。
- 组合式函数（Composables）统一放在 `src/composables/` 目录，文件名以 `use` 开头，例如 `useWindowState.ts`。
- 禁止在 `<template>` 中编写业务逻辑，复杂逻辑须提取到 `<script setup>` 或 Composable 中。

### Rust（Tauri Command）

- 每个 Tauri command 函数须附带中文注释说明其功能与参数含义。
- 错误须通过 `Result<T, String>` 或自定义错误类型返回，禁止 `unwrap()` / `expect()` 出现在非初始化代码中。

---

## 三、架构与模块化原则

### 目录结构约定

```
src/
├── assets/          # 静态资源
├── components/      # 全局通用组件（无业务逻辑）
├── composables/     # 组合式函数
├── router/          # vue-router 路由配置
├── stores/          # pinia store（按业务域拆分文件）
├── views/           # 页面级组件（与路由一一对应）
└── utils/           # 纯函数工具
src-tauri/
└── src/
    └── commands/    # Tauri command 按功能模块拆分文件
```

### 原则

1. **单一职责**：每个组件、函数、Store 只负责一件事。
2. **组件分层**：`components/` 只放与业务无关的 UI 组件；业务组件放在对应 `views/` 子目录内。
3. **Store 按域拆分**：每个业务域对应一个 pinia store 文件，禁止创建单一全局大 store。
4. **前后端边界清晰**：前端不处理系统级逻辑，系统级操作统一通过 Tauri command 调用；Tauri command 不处理 UI 状态。
5. **禁止硬编码**：常量统一定义在 `src/utils/constants.ts` 或各模块的 `constants.ts` 中。
6. **禁止 fallback 掩盖错误**：代码应快速失败（Fail-Fast），不得添加吞掉错误的 try-catch 或默认值来掩盖异常。

---

## 四、Git 工作流规范

### 分支命名

| 类型 | 命名格式 | 示例 |
|---|---|---|
| 功能开发 | `feat/功能名` | `feat/system-tray` |
| 问题修复 | `fix/问题描述` | `fix/window-resize-crash` |
| 重构 | `refactor/模块名` | `refactor/store-split` |
| 文档 | `docs/内容` | `docs/update-readme` |

### 工作流约定

1. 禁止直接向 `main` / `master` 分支提交代码。
2. 每个功能或修复对应一个独立分支，完成后通过 PR/MR 合并。
3. 合并前须确保本地已同步最新 `main` 分支（`git rebase` 或 `git merge`）。

---

## 五、Commit 提交规范

遵循 Angular Commit Message 规范，**提交信息使用中文**。

### 格式

```
<类型>(<范围>): <简短描述>

[可选正文]

[可选脚注]
```

### 类型说明

| 类型 | 说明 |
|---|---|
| `feat` | 新增功能 |
| `fix` | 修复问题 |
| `refactor` | 重构（不新增功能、不修复问题） |
| `style` | 代码格式调整（不影响逻辑） |
| `perf` | 性能优化 |
| `chore` | 构建流程、依赖更新等杂项 |
| `docs` | 文档变更 |
| `revert` | 回滚提交 |

## 六、跨平台开发规则

- 优先采用跨平台通用方案，包括Rust后端逻辑和前端浏览器内核兼容逻辑
- 优先使用成熟第三方依赖，避免重复造轮子
- 新方案必须先评估 Windows / macOS / Linux 一致性
- 依赖选型优先考虑维护状态、社区活跃度、许可证、跨平台兼容性
- 除非明确要求，否则不要引入平台专属实现
- 改动前先阅读现有架构与技术栈，再设计实现路径