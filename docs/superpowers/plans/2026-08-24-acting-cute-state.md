# 撒娇动作状态替换实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将桌宠的 `waving` 动画状态彻底替换为 `acting_cute`，让新生成、导入、播放和插件触发的动作都表现为“撒娇”。

**Architecture:** 保留现有四状态数据驱动结构，只替换状态键和对应的动作契约，不增加新的动画层或兼容别名。TypeScript 的状态目录驱动创建器和桌宠播放端，Rust 的同名状态目录驱动生成、组装和保存；两端通过 `acting_cute`、`actingCuteFrames` 和 `acting_cute_frames` 对应连接。

**Tech Stack:** React 19、TypeScript、Vitest、Testing Library、Tauri 2、Rust、Cargo。

---

## 文件地图

- 状态公共契约：`src/types/pet.ts`、`src/types/__tests__/pet.test.ts`
- 桌宠触发与插件：`src/windows/Pet/index.tsx`、`src/store/__tests__/petStore.test.ts`、`src/lib/bundled-plugins.ts`、`plugins/schedule-reminder.js`、`plugins/claude-code-progress.js`、`src/lib/__tests__/plugin-sandbox.test.ts`
- 创建器前端：`src/windows/Creator/index.tsx`、`src/windows/Creator/steps/types.ts`、`src/windows/Creator/steps/StateGenerationStep.tsx`、`src/windows/Creator/steps/ManualFramePickerStep.tsx`
- 创建器测试：`src/windows/Creator/steps/__tests__/StateGenerationStep.test.tsx`、`src/windows/Creator/steps/__tests__/ManualFramePickerStep.test.tsx`、`src/windows/Creator/steps/__tests__/PreviewStep.test.tsx`、`src/windows/Creator/steps/__tests__/SaveStep.test.tsx`
- Rust 状态和提示词：`src-tauri/src/commands/generation/types.rs`、`src-tauri/src/commands/generation/prompts.rs`
- Rust 生成/导入/保存：`src-tauri/src/commands/generation/mod.rs`、`src-tauri/src/commands/generation/run.rs`、`src-tauri/src/commands/pet.rs`、`src-tauri/src/models.rs`
- Rust 测试：上述 Rust 文件中的 `#[cfg(test)]` 模块，以及 `src-tauri/src/commands/generation/sprite.rs` 中仅涉及旧动作文案的注释。

## 任务 1：先锁定 TypeScript 状态契约

**Files:**
- Modify: `src/types/__tests__/pet.test.ts`
- Modify: `src/store/__tests__/petStore.test.ts`
- Modify: `src/lib/__tests__/pet-commands.test.ts`
- Modify: `src/lib/__tests__/plugin-sandbox.test.ts`
- Modify: `src/types/pet.ts`

- [ ] **Step 1: 把前端契约测试改成期望 `acting_cute`**

在 `src/types/__tests__/pet.test.ts` 中，将状态集合断言和 `Pet.states` fixture 改为：

```ts
expect(PET_STATES).toContain('acting_cute');
expect(PET_STATES).not.toContain('waving');
expect(PET_STATES).toHaveLength(4);

states: { idle: META, sleeping: META, acting_cute: META, working: META },
```

把 `src/store/__tests__/petStore.test.ts`、`src/lib/__tests__/pet-commands.test.ts` 中的 fixture 和状态断言改为 `acting_cute`；把 `src/lib/__tests__/plugin-sandbox.test.ts` 的插件代码与回调断言改为：

```ts
sandbox.loadPlugin(`pet.setState('acting_cute');`);
expect(onSetState).toHaveBeenCalledWith('acting_cute');
```

- [ ] **Step 2: 运行前端契约测试，确认失败原因是生产类型仍为 `waving`**

Run from `desktop-pet`:

```powershell
npx vitest run src/types/__tests__/pet.test.ts src/store/__tests__/petStore.test.ts src/lib/__tests__/pet-commands.test.ts src/lib/__tests__/plugin-sandbox.test.ts
```

Expected: FAIL，错误集中在 `acting_cute` 不是当前 `PetState` 或 `PET_STATES` 仍包含 `waving`；如果出现无关导入错误，先修正测试本身后再继续。

- [ ] **Step 3: 实现新的 TypeScript 状态目录**

在 `src/types/pet.ts` 中使用以下契约：

```ts
export type PetState = 'idle' | 'sleeping' | 'acting_cute' | 'working';
export const PET_STATES: PetState[] = ['idle', 'sleeping', 'acting_cute', 'working'];

export const PET_STATE_LABELS: Record<PetState, string> = {
  idle: '待机',
  sleeping: '睡觉',
  acting_cute: '撒娇',
  working: '工作',
};

export const PET_STATE_CATALOG: readonly PetStateDefinition[] = [
  { key: 'idle', label: PET_STATE_LABELS.idle, delayMs: 150 },
  { key: 'sleeping', label: PET_STATE_LABELS.sleeping, delayMs: 200 },
  { key: 'acting_cute', label: PET_STATE_LABELS.acting_cute, delayMs: 110 },
  { key: 'working', label: PET_STATE_LABELS.working, delayMs: 120 },
];
```

- [ ] **Step 4: 运行契约测试确认通过**

```powershell
npx vitest run src/types/__tests__/pet.test.ts src/store/__tests__/petStore.test.ts src/lib/__tests__/pet-commands.test.ts src/lib/__tests__/plugin-sandbox.test.ts
```

Expected: PASS。

- [ ] **Step 5: 提交状态契约变更**

```powershell
git add src/types src/store/__tests__/petStore.test.ts src/lib/__tests__/pet-commands.test.ts src/lib/__tests__/plugin-sandbox.test.ts
git commit -m "refactor: replace waving state in frontend contract"
```

## 任务 2：替换 Rust 状态目录和生成动作提示词

**Files:**
- Modify: `src-tauri/src/commands/generation/types.rs`
- Modify: `src-tauri/src/commands/generation/prompts.rs`

- [ ] **Step 1: 先把 Rust 状态和提示词测试改成新契约**

在 `types.rs` 的状态目录测试中期望：

```rust
vec!["idle", "sleeping", "acting_cute", "working"]
```

在 `prompts.rs` 的行提示词测试中改用：

```rust
let state = state_definition("acting_cute").unwrap();
```

并增加一个明确的行为断言：

```rust
assert!(prompt.contains("hands close to the face or chest"));
assert!(prompt.contains("No hearts, text, symbols, motion lines"));
assert!(!prompt.to_lowercase().contains("wave"));
```

- [ ] **Step 2: 运行 Rust 相关测试确认失败**

Run from `desktop-pet\src-tauri`:

```powershell
cargo test phase_one_catalog_has_four_states_and_fixed_timing
cargo test row_prompt
```

Expected: FAIL，因为当前目录没有 `acting_cute`，且当前提示词仍是挥手动作。

- [ ] **Step 3: 替换 Rust 状态定义**

在 `types.rs` 中把第三个 `StateDefinition` 改为 `acting_cute`，保留 110ms 播放间隔，并使用以下动作约束：

```rust
StateDefinition {
    key: "acting_cute",
    label: "撒娇",
    delay_ms: 110,
    facing: "forward (front-facing, exactly as in the canonical base image)",
    action: "a cute, affectionate 8-frame cycle with both hands held close to the face or chest; the head and upper body sway gently left and right in tiny continuous increments, with one brief shy blink on a single frame; keep the character planted in place and finish in a seamless loop",
    requirements: "Both hands stay close to the face or chest in every frame. The motion is small, continuous, and centered: no greeting gesture, large arm lift, jumping, translation, head turn, or change of facing direction. No hearts, text, symbols, motion lines, sparkles, particles, glow, speech bubbles, or other detached effects. Preserve the same character identity, scale, baseline, camera, and background across all 8 frames.",
},
```

同时删除 idle、sleeping、working 约束中旧状态名和挥手专用措辞，统一改成“no greeting gesture”等通用限制，避免生产源码残留旧动作名；其他状态动作不变。

- [ ] **Step 4: 运行 Rust 状态和提示词测试确认通过**

```powershell
cargo test phase_one_catalog_has_four_states_and_fixed_timing
cargo test row_prompt
```

Expected: PASS。

- [ ] **Step 5: 提交 Rust 状态与提示词变更**

```powershell
git add src-tauri/src/commands/generation/types.rs src-tauri/src/commands/generation/prompts.rs
git commit -m "feat: define acting cute sprite generation"
```

## 任务 3：替换 Rust 生成、导入和保存链路

**Files:**
- Modify: `src-tauri/src/commands/generation/mod.rs`
- Modify: `src-tauri/src/commands/generation/run.rs`
- Modify: `src-tauri/src/commands/pet.rs`
- Modify: `src-tauri/src/models.rs`

- [ ] **Step 1: 先更新 Rust 生成/保存测试中的状态名和字段名**

把这些测试数据和路径全部改为 `acting_cute`：

```rust
["idle", "sleeping", "acting_cute", "working"]
```

把手动导入和合并测试中使用的参数改成 `acting_cute_cells` / `acting_cute_frames`，并把断言路径改为 `acting_cute.png`。`models.rs` 的 `Pet` fixture 也必须包含 `acting_cute` 而不是 `waving`。

- [ ] **Step 2: 运行 Rust 全部测试，确认签名不匹配并失败**

```powershell
cargo test
```

Expected: FAIL，错误应指向旧的 `waving_*` 函数参数、状态文件或测试 fixture；不应通过修改测试断言来掩盖生产签名不一致。

- [ ] **Step 3: 替换生成和保存函数的状态参数**

在 `src-tauri/src/commands/generation/mod.rs` 中执行同一契约替换：

```rust
pub async fn save_combined_sprite_sheet(
    // ...
    acting_cute_frames: u32,
    // ...
)

let rows: [(&str, u32, u32); 4] = [
    ("idle", idle_frames, 150),
    ("sleeping", sleeping_frames, 200),
    ("acting_cute", acting_cute_frames, 110),
    ("working", working_frames, 120),
];
```

同样把 `write_frame_selections_to_dir`、`stage_frame_selections_at`、`stage_frame_selections` 和 `save_frame_selections` 的 `waving_cells` 参数改成 `acting_cute_cells`，对应 `state_entries` 使用 `("acting_cute", acting_cute_cells, 110)`。只改状态键和参数名，不改裁剪、校验或 PNG 编码逻辑。

在 `run.rs`、`mod.rs` 中所有状态循环和 `validate_state_name` 相关测试使用 `acting_cute`；`models.rs` 仅更新四状态测试 fixture。

把 `src-tauri/src/commands/generation/sprite.rs` 中注释里的 `waving arm reaching out` 改成不绑定具体动作的 `wide-outlier frame reaching outward`，确保运行时代码和源码注释都不再使用旧状态名。

- [ ] **Step 4: 更新宠物最终保存的四个 PNG 文件**

在 `src-tauri/src/commands/pet.rs` 的 `read_selected_pngs` 中使用：

```rust
["idle", "sleeping", "acting_cute", "working"]
```

让原子保存测试同时检查 `pets/<id>/acting_cute.png`，并确保缺失该文件时仍会在提交前失败且不留下正式宠物目录。

- [ ] **Step 5: 运行 Rust 全部测试确认通过**

```powershell
cargo test
```

Expected: PASS。

- [ ] **Step 6: 提交 Rust 生成和保存链路变更**

```powershell
git add src-tauri/src/commands/generation/mod.rs src-tauri/src/commands/generation/run.rs src-tauri/src/commands/pet.rs src-tauri/src/models.rs
git commit -m "refactor: rename sprite files to acting cute"
```

## 任务 4：替换创建器前端的生成配置、选帧和预览

**Files:**
- Modify: `src/windows/Creator/steps/types.ts`
- Modify: `src/windows/Creator/steps/StateGenerationStep.tsx`
- Modify: `src/windows/Creator/steps/ManualFramePickerStep.tsx`
- Modify: `src/windows/Creator/index.tsx`
- Modify: `src/windows/Creator/steps/__tests__/StateGenerationStep.test.tsx`
- Modify: `src/windows/Creator/steps/__tests__/ManualFramePickerStep.test.tsx`
- Modify: `src/windows/Creator/steps/__tests__/PreviewStep.test.tsx`
- Modify: `src/windows/Creator/steps/__tests__/SaveStep.test.tsx`

- [ ] **Step 1: 更新创建器测试期望并运行失败测试**

把测试中的状态序列、fixture、选帧参数、路径和配置字段统一改为：

```ts
['idle', 'sleeping', 'acting_cute', 'working']
actingCuteFrames: 8
actingCuteCells: Array.from({ length: 8 }, (_, col) => ({ col, row: 2 }))
```

在 `StateGenerationStep.test.tsx` 中，批量生成和重试断言必须看到 `acting_cute`；在 `PreviewStep.test.tsx` 中，路径必须包含 `acting_cute.png`；在 `ManualFramePickerStep.test.tsx` 中，保存 invoke 的参数必须包含 `actingCuteCells`。

Run:

```powershell
npx vitest run src/windows/Creator/steps/__tests__
```

Expected: FAIL，原因是创建器生产代码仍生成 `waving` 配置或调用旧参数。

- [ ] **Step 2: 修改生成配置接口和状态生成步骤**

在 `src/windows/Creator/steps/types.ts` 与 `StateGenerationStep.tsx` 中将 `wavingFrames` 改为 `actingCuteFrames`，并在 `buildGeneratedConfig` 返回：

```ts
actingCuteFrames: 8,
```

`StateGenerationStep` 仍通过 `PET_STATES` 驱动 4 个状态，因此只需确保类型编译和生成配置字段使用新名称；进度总数保持 4，播放间隔从 `PET_STATE_CATALOG` 获取。

- [ ] **Step 3: 修改手动选帧步骤和 Creator 连接**

在 `ManualFramePickerStep.tsx` 中：

```ts
actingCuteFrames?: number;

const ACTION_COLORS: Record<PetState, string> = {
  idle: '#4f8ef7',
  sleeping: '#48bb78',
  acting_cute: '#ed8936',
  working: '#e53e3e',
};
```

将 `frameCounts`、依赖数组、`handleSave` 的 invoke 参数和所有选帧对象使用 `acting_cute` / `actingCuteCells`。在 `Creator/index.tsx` 将生成配置透传字段改为 `actingCuteFrames`。

- [ ] **Step 4: 运行创建器测试确认通过**

```powershell
npx vitest run src/windows/Creator/steps/__tests__
```

Expected: PASS，且预览路径只出现新状态文件名。

- [ ] **Step 5: 提交创建器变更**

```powershell
git add src/windows/Creator
git commit -m "refactor: use acting cute in creator flow"
```

## 任务 5：替换桌宠触发入口和所有前端 fixture

**Files:**
- Modify: `src/windows/Pet/index.tsx`
- Modify: `src/lib/bundled-plugins.ts`
- Modify: `plugins/schedule-reminder.js`
- Modify: `plugins/claude-code-progress.js`

- [ ] **Step 1: 修改桌宠和内置插件触发状态**

在 `src/windows/Pet/index.tsx` 中把鼠标移入处理改为：

```tsx
onMouseEnter={() => { if (!showPicker) setPetState('acting_cute'); }}
```

在两个内置插件源码和 `plugins/*.js` 示例中，把原先的 `pet.setState('waving')` 改成 `pet.setState('acting_cute')`。事件时序不变：提醒和任务完成仍在原入口触发一次非 idle 状态。

- [ ] **Step 2: 运行全量前端测试确认没有旧状态依赖**

```powershell
npx vitest run
```

Expected: PASS。

- [ ] **Step 3: 搜索生产代码中的旧状态引用**

```powershell
rg -n -i "waving|wavingFrames|wavingCells" src src-tauri plugins
```

Expected: 无输出。设计文档和历史计划中的 `waving` 说明不属于运行时代码，不纳入此检查。

- [ ] **Step 4: 提交触发入口变更**

```powershell
git add src/windows/Pet/index.tsx src/lib/bundled-plugins.ts plugins src/store/__tests__ src/lib/__tests__
git commit -m "feat: trigger acting cute state"
```

## 任务 6：清理旧应用数据并完成集成验证

**Files:**
- No source files; operate only on the verified desktop-pet application data directory.

- [ ] **Step 1: 只读定位应用数据目录**

根据 `src-tauri/tauri.conf.json` 的 identifier `com.administrator.desktop-pet`，检查以下明确路径，不使用递归删除或未解析的通配符：

```powershell
$appDataPath = Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'com.administrator.desktop-pet'
Resolve-Path $appDataPath -ErrorAction SilentlyContinue
Get-ChildItem -LiteralPath (Join-Path $appDataPath 'pets') -Force -ErrorAction SilentlyContinue
```

确认输出的实际绝对路径与 `$appDataPath\pets` 完全一致后，执行以下有界删除；保留应用设置和项目目录。删除前记录实际绝对路径，删除后检查 `pets` 不存在或为空，并向用户报告删除的精确路径：

```powershell
$petsPath = [IO.Path]::GetFullPath((Join-Path $appDataPath 'pets'))
$expectedPetsPath = [IO.Path]::GetFullPath((Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'com.administrator.desktop-pet\pets'))
if ($petsPath -ne $expectedPetsPath) { throw "Refusing to delete unexpected path: $petsPath" }
if (Test-Path -LiteralPath $petsPath -PathType Container) {
    Remove-Item -LiteralPath $petsPath -Recurse -Force
}
Test-Path -LiteralPath $petsPath
```

- [ ] **Step 2: 运行所有自动化验证**

From `desktop-pet`:

```powershell
npx vitest run
npm run build
```

From `desktop-pet\src-tauri`:

```powershell
cargo test
```

Expected: Vitest、TypeScript build 和 Cargo test 全部成功。

- [ ] **Step 3: 启动 Tauri 做一次手动冒烟验证**

```powershell
npm run tauri dev
```

在创建器中生成或导入一只新宠物，确认最终目录包含 `acting_cute.png` 而不包含 `waving.png`；关闭创建器后将鼠标移入桌宠，确认它切换到“撒娇”状态，任务完成和定时提醒也使用同一状态。退出开发进程后继续下一步。

- [ ] **Step 4: 做最终源码契约检查**

```powershell
rg -n -i "waving|wavingFrames|wavingCells" src src-tauri plugins
rg -n "acting_cute|actingCuteFrames|actingCuteCells" src src-tauri plugins
git diff --check
git status --short
```

Expected: 第一条无输出；第二条覆盖状态定义、生成/保存链路、创建器和触发入口；`git diff --check` 无空白错误。保留用户已有的 `template.webp` 未跟踪文件，不加入提交。

- [ ] **Step 5: 提交最终验证结果**

```powershell
git add src src-tauri plugins
git commit -m "test: verify acting cute state replacement"
```

若最终没有待提交源码变更，不创建空提交，只在交付说明中报告验证结果和已清理的数据路径。
