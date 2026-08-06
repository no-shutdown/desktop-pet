# Canonical Base 精灵图生成重构设计

> 状态：设计已获用户确认，待用户审阅文档后进入实现计划。

## 背景

当前项目的生成流程把角色描述和动画状态组合成提示词，然后为 `idle`、`walking`、`waving`、`working` 四个状态分别调用文字生图。每个状态独立生成，虽然提示词要求“same character design”，但没有真正的参考图输入，因此脸部、比例、服装、材质和配色可能在状态之间漂移。

当前工作区已经包含精灵图 PNG、Canvas 动画和手动选帧的未提交重构。新的工作必须保留这些用户改动，并在现有四状态格式上增加 canonical base 驱动的生成流程。

本设计参考 OpenAI `hatch-pet` 的流程：先生成并确认 canonical base，再让每个动画行使用该参考图；提示词保持简洁、状态化、面向精灵图生产，较长的 QA 规则由程序验证承担。

## 目标

1. 生成一张稳定的 canonical base 角色参考图。
2. 生成每个动画状态时，将 canonical base 作为图像参考输入。
3. SiliconFlow 作为默认云端通道，Local SD 作为本地高一致性通道。
4. 支持 Base 重试和单状态重试，不必重新生成整组动画。
5. 保持现有 `pet.json` 和四状态宠物兼容。
6. 将状态、动作提示词、负面规则和播放参数改成数据驱动，为未来九状态扩展保留接口。
7. 生成中间文件与正式宠物文件分离，取消任务不会污染 `pets/<pet_id>`。

## 非目标

本阶段不实现以下内容：

- Pollinations 图生图适配。
- hatch-pet 的九状态 Creator UI。
- `192×208` 新精灵图格式；第一阶段保持 `128×128`，未来通过版本化格式升级。
- 自动视觉模型 QA。
- IP-Adapter 或 ControlNet 的配置界面；Local SD 先使用原生 img2img，扩展能力后续接入。

## 设计决策

### 1. 任务分为 Base 和 Row 两类

生成流程如下：

```text
角色描述
  → 生成 canonical base
  → 用户确认或重试
  → 生成 idle 行
  → 生成 walking 行
  → 生成 waving 行
  → 生成 working 行
  → 去背景、尺寸验证、手动选帧
  → 保存正式宠物
```

Base 只包含一个完整、居中的中性姿势，不包含网格、帧序列或动作。每个 Row 直接输出一个包含 8 帧的水平动画条。现有外部组合精灵图导入流程仍可继续接收 `4×2` 等网格格式，并在导入时转换为水平条。

后端统一抽象为两个能力：

```text
generate_base(prompt, provider_config) -> base image
generate_row(prompt, reference_image, state, provider_config) -> row image
```

如果参考图调用失败，系统必须报告错误，不能静默退回纯文字生图；否则会破坏身份锁定的核心保证。

### 2. SiliconFlow 和 Local SD 双通道

#### SiliconFlow

- Base 阶段使用现有文字生图能力。
- Row 阶段使用支持图生图的模型，并把 Base 编码为 `image` 字段。
- 初始参考图模型列表包含 `Qwen/Qwen-Image-Edit-2509` 和 `Kwai-Kolors/Kolors`。
- 现有的 `Tongyi-MAI/Z-Image-Turbo` 保留为 Base 文字生图模型。
- 前端设置将 Base 模型和参考图模型分开，避免把不支持图生图的模型用于 Row 阶段。

SiliconFlow 的图片生成接口支持 Base64 或 URL 图像输入，并对部分模型提供 `image2`、`image3` 输入。

#### Local SD

- Base 阶段调用 `/sdapi/v1/txt2img`。
- Row 阶段调用 `/sdapi/v1/img2img`，通过 `init_images` 传入 Base。
- 增加 `denoising_strength` 设置，默认值为 `0.55`，允许用户在 `0.35` 到 `0.75` 之间调整。
- 如果未来检测到 IP-Adapter 或 ControlNet 扩展，可在同一 Provider 接口中增加可选参数，但不作为第一阶段前置条件。

### 3. 中间任务目录

每次生成创建一个独立的运行目录：

```text
app_data_dir/runs/<run_id>/
├─ manifest.json
├─ base.png
└─ rows/
   ├─ idle.png
   ├─ walking.png
   ├─ waving.png
   └─ working.png
```

`manifest.json` 保存以下信息：

```json
{
  "version": 1,
  "runId": "uuid",
  "provider": "siliconflow",
  "base": { "status": "complete", "path": "base.png", "attempts": 1 },
  "states": {
    "idle": { "status": "pending", "path": "rows/idle.png", "attempts": 0 },
    "walking": { "status": "pending", "path": "rows/walking.png", "attempts": 0 },
    "waving": { "status": "pending", "path": "rows/waving.png", "attempts": 0 },
    "working": { "status": "pending", "path": "rows/working.png", "attempts": 0 }
  },
  "frameW": 128,
  "frameH": 128,
  "frameCount": 8,
  "chromaKey": "#FF00FF"
}
```

只有用户最终保存时，才把处理后的 PNG 写入：

```text
pets/<pet_id>/pet.json
pets/<pet_id>/idle.png
pets/<pet_id>/walking.png
pets/<pet_id>/waving.png
pets/<pet_id>/working.png
```

已存在的 `pets/*/raw` 目录不自动迁移或删除，避免影响用户已有数据。

### 4. Creator 交互

Creator 从：

```text
上传 → 分析 → 生成全部动画 → 导入精灵图 → 预览 → 保存
```

调整为：

```text
上传 → 分析 → 生成 Base → 确认 Base
     → 生成动画状态 → 手动选帧 → 预览 → 保存
```

生成界面需要提供：

- Base 预览。
- Base 重新生成按钮。
- 当前状态、总进度和错误信息。
- 单状态重试按钮。
- SiliconFlow 参考图模型选择。
- Local SD img2img 参数设置。

后端命令按运行任务拆分为以下职责：

```text
generate_base_preview      创建运行任务并生成 base.png
generate_state_row         使用 base.png 生成指定状态
assemble_run_preview       将已完成状态合并为 Creator 预览
save_frame_selections      从运行任务写入正式宠物 PNG
discard_generation_run     删除未完成运行任务
```

外部导入的组合精灵图仍可走现有导入流程，不强制经过 Base 生成。

## 提示词设计

提示词不复制 `hatch-pet` 全部文档，而是采用同样的分层结构：

```text
角色身份
+ 风格契约
+ 精灵图布局契约
+ 状态动作
+ 状态专属限制
+ Chroma key 契约
+ 负面关键词
```

### Base 模板

```text
Create one clean full-body reference sprite for a desktop pet.

Pet identity: {user_description}
Style: {style_contract}

Create exactly one centered, complete character in a neutral relaxed pose.
Keep the entire body visible with a compact readable silhouette.
Preserve the same face, proportions, markings, palette, materials,
clothing, accessories, and prop design for all later animation states.

Use a perfectly flat {chroma_name} {chroma_hex} chroma-key background.
Keep the chroma-key color out of the character, props, highlights, and effects.

No extra characters, scenery, text, labels, logos, watermark, UI, grid,
border, checkerboard transparency, shadow, glow, particles, or detached effects.
```

### Row 模板

```text
Create one horizontal animation strip for the same desktop pet.
Use the attached canonical base image as the identity reference.

Output exactly {frame_count} full-body frames from left to right on a flat
{chroma_name} {chroma_hex} chroma-key background.
Treat the image as invisible equal-width slots: one centered complete pose
per slot, evenly spaced, no overlap, clipping, empty slots, labels, or borders.

Preserve the same pet in every frame: same face, silhouette, proportions,
markings, palette, materials, style, clothing, accessories, and props.
Keep apparent scale and baseline stable across frames.

State action: {state_action}
State requirements: {state_requirements}

No text, logos, UI, scenery, grid, guide marks, checkerboard, shadows,
glow, motion blur, speed lines, dust, detached effects, stray pixels,
cropped limbs, extra characters, or chroma-key colors inside the pet.
```

### 四状态动作契约

```text
idle:
calm low-distraction resting loop, subtle breathing, tiny blink,
slight head or body bob, nearly unchanged silhouette and planted baseline.
No walking, waving, working, jumping, emotional reactions, large gestures,
item interaction, or new props.

walking:
rightward walking cycle, alternating legs and opposite arm swing,
clear directional cadence, stable body scale and baseline.
No speed lines, dust, shadows, motion trails, or detached effects.

waving:
friendly greeting shown only through the raised paw, hand, wing, or limb.
No wave marks, motion arcs, lines, sparkles, symbols, or floating effects.

working:
focused active-task processing, typing, thinking, scanning, or purposeful
hand/paw motion; not literal foot-running. Only use props already present
in the canonical base. No UI, code, papers, symbols, or detached props.
```

### Chroma key

候选颜色沿用 hatch-pet 的思路：洋红、青色、黄色、蓝色、橙色、绿色。根据用户上传的角色参考图采样颜色，选择与角色颜色距离最大的候选色；没有参考图时使用洋红作为 fallback。后端使用目标颜色距离而不是当前的亮暗背景二分法进行去背景，并将完全透明像素的 RGB 归零。

### 状态目录

当前状态目录包含：

```text
idle     8 帧，150ms，低幅度待机
walking  8 帧，100ms，向右行走
waving   8 帧，110ms，肢体挥手
working  8 帧，120ms，工作/处理任务
```

状态目录是后续扩展九状态的边界。未来增加状态时只需要新增动作、限制、帧数和播放参数，不改变生成任务和保存接口。

## 验证策略

每个状态正式保存前验证：

1. PNG 可解码。
2. 宽度等于 `frameWidth × frameCount`。
3. 高度等于 `frameHeight`。
4. 帧数和 manifest 一致。
5. 背景色被正确转换为透明。
6. 完全透明像素没有 RGB 残留。
7. 没有空帧、明显裁切或跨槽位内容。

角色身份一致性不使用简单像素相等判断，而由 canonical base 预览、状态预览和用户确认共同完成。

## 测试策略

### Rust

- Base 和 Row 提示词的身份锁定、状态动作和禁止元素。
- 状态目录和帧参数。
- Chroma key 颜色选择、颜色距离和透明像素归零。
- SiliconFlow Base/Row 请求体。
- Local SD `txt2img`/`img2img` 请求体。
- 运行任务 manifest、状态重试和取消清理。
- 精灵图尺寸、帧数和 PNG 验证。

### React/TypeScript

- Base 生成、确认、重新生成。
- 单状态生成、重试和进度展示。
- 任务失败时不错误降级为文字生图。
- 现有手动选帧、预览和保存流程。
- 旧 `pet.json` 加载和旧 `128×128` 动画播放。

## 验收标准

1. SiliconFlow 模式可以完成 Base → Row → 保存，并且 Row 请求包含 Base 图像。
2. Local SD 模式可以完成 Base → img2img Row → 保存。
3. Base 失败只影响 Base，Row 失败只影响对应状态。
4. 用户取消生成不会在 `pets/<pet_id>` 留下未完成宠物。
5. 现有旧宠物无需迁移即可正常加载和播放。
6. 四状态的提示词包含身份锁定、布局规则、状态限制和去背景约束。
7. 所有新增逻辑有对应自动化测试。

## 参考资料

- [OpenAI hatch-pet SKILL.md](https://raw.githubusercontent.com/openai/skills/main/skills/.curated/hatch-pet/SKILL.md)
- [hatch-pet 的提示词与状态契约](https://raw.githubusercontent.com/openai/skills/main/skills/.curated/hatch-pet/scripts/prepare_pet_run.py)
- [SiliconFlow 图片生成 API](https://api-docs.siliconflow.cn/docs/api/images-generations-post)
- [AUTOMATIC1111 Stable Diffusion WebUI](https://github.com/AUTOMATIC1111/stable-diffusion-webui)
