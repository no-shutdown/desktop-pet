# 撒娇动作状态替换设计

## 背景

桌宠目前有四个动画状态：`idle`、`sleeping`、`waving`、`working`。其中 `waving` 同时被前端状态类型、创建器状态目录、Rust 生成目录、生成提示词、鼠标悬停行为、内置插件和测试使用。

本次改动把 `waving` 完整替换为 `acting_cute`，让用户可见的动作变成“撒娇”，并放弃旧 `waving` 宠物数据的兼容迁移。

## 目标

- 四状态目录统一为 `idle`、`sleeping`、`acting_cute`、`working`。
- “撒娇”动画为 8 帧连续循环：双手靠近脸或胸口，头部或身体轻轻左右晃，偶尔眨眼，角色保持原地和固定朝向。
- 禁止生成爱心、文字、动作线、漂浮特效等脱离角色的元素。
- 生成、预览、导入、保存、播放、插件和测试使用同一个 `acting_cute` 状态键。
- 鼠标移入桌宠、定时提醒和任务完成事件触发 `acting_cute`。

## 非目标

- 不增加第五个动画状态。
- 不保留 `waving` 到 `acting_cute` 的运行时别名或数据迁移。
- 不改变其他三个状态的动作、帧数或播放时序。
- 不删除项目源码、项目插件源码或应用之外的用户文件。

## 状态与生成契约

TypeScript 和 Rust 共享以下四状态顺序：

```text
idle -> sleeping -> acting_cute -> working
```

`acting_cute` 使用原 `waving` 状态的 110ms 帧间隔，避免无关的播放节奏变化。Rust 状态定义的动作提示词要求：

- 双手靠近脸或胸口，动作小幅、连续、循环；
- 头部或身体轻轻左右晃，允许单帧眨眼；
- 角色比例、基线、位置、朝向和身份在 8 帧中保持稳定；
- 不挥手、不抬臂做问候动作、不出现爱心、文字、符号、动作线、粒子或漂浮物。

生成行、组合预览和最终保存的文件名由 `waving.png` 改为 `acting_cute.png`，最终宠物元数据的状态键也使用 `acting_cute`。

## 受影响组件

- `src/types/pet.ts`：状态类型、目录和中文标签。
- `src/windows/Pet/index.tsx`：鼠标移入触发状态。
- `src/lib/bundled-plugins.ts` 与 `plugins/*.js`：内置提醒和任务完成触发状态。
- `src/windows/Creator/steps/*`：生成配置、状态面板、手动选帧和预览。
- `src-tauri/src/commands/generation/types.rs`：状态定义和生成提示词动作契约。
- `src-tauri/src/commands/generation/mod.rs`、`src-tauri/src/commands/generation/run.rs`、`src-tauri/src/commands/generation/sprite.rs`：状态文件和组装流程。
- `src-tauri/src/commands/pet.rs`、`src-tauri/src/models.rs`：保存和读取的四状态契约。
- 前端、Rust 现有测试：所有 `waving` 断言、测试数据和字段名改为 `acting_cute` / `actingCute`。

## 旧数据处理

不实现兼容迁移。旧宠物的 `waving` 状态不再是有效状态；实现验证时先定位应用数据目录，仅在确认目标为桌宠旧宠物数据目录后清理旧宠物数据。项目源码和其他用户文件不在清理范围内。清理后用户需要重新创建桌宠。

## 测试与验收

先写并运行失败测试，确认它们因为旧状态契约仍存在而失败；随后实现最小改动并验证通过：

1. TypeScript 状态测试确认四状态目录包含 `acting_cute`、不包含 `waving`。
2. Rust 状态目录和提示词测试确认生成键为 `acting_cute`，提示词包含撒娇动作要求且不含挥手语义。
3. 创建器、手动选帧、预览、保存和插件测试确认使用新状态键。
4. 运行前端 Vitest、Rust 单元测试和 `npm run build`。
5. 启动 Tauri 开发模式，确认鼠标移入和任务完成时加载 `acting_cute.png`。

验收标准是：新建宠物只生成并保存 `acting_cute.png`；应用内不再引用 `waving` 状态；三个触发入口都显示撒娇动作；所有自动化测试和构建通过。
