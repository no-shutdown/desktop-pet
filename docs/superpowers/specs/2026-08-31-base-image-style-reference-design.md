# 基础图风格参考图设计

日期：2026-08-31

## 背景

创建器第 3 步目前只把角色描述发送给基础图生成服务。用户已经可以上传人物原图，但该图当前主要用于选择抠图色键，基础图生成本身依赖文字描述，因此很难稳定复现用户希望的画风、配色和可爱程度。

## 目标

- 在“生成基础图像”步骤增加一张可选的风格参考图。
- 将风格参考图严格解释为纯风格输入：参考线稿、配色、材质、阴影、比例和整体气质，但不复制其中的角色、服装、姿势、背景、构图或道具。
- 角色身份优先来自创建器原始人物图和角色描述。
- 未上传风格参考图时，现有基础图生成请求和结果保持不变。
- 风格参考图只参与基础图生成，不进入第 4 步状态行生成；状态行仍只使用已确认的 canonical base。
- 对不支持可靠多图纯风格参考的模型明确报错或提示，不能静默丢弃用户选择的图片。

## 非目标

- 不把风格参考图持久化到宠物资源或生成 run manifest；它只在当前创建器流程中作为基础图请求输入。
- 不改变已有卡通图与真实人物图的 `SourceStyle` 规则。
- 不为 Local SD 引入 IP-Adapter、ControlNet 或其他扩展依赖。
- 不让风格参考图影响后续动画动作、朝向、桌面或笔记本电脑提示词。

## 设计

### 1. 创建器状态与界面

在 `WizardData` 增加可空的 `styleReferenceDataUrl`，初始值为 `null`。创建器开始新的 AI 生成、放弃生成或重置时清空该字段；从基础图步骤返回再进入时保留它，便于重新生成时继续使用。

`GenerateStep` 增加“风格参考图（可选）”区域：

- 接受 JPG、PNG、WEBP；使用 `FileReader.readAsDataURL`，沿用主参考图的本地数据 URL 方式。
- 无图片时展示上传入口和说明；有图片时展示预览与“移除”操作。
- 选择或移除后通过 `onStyleReferenceChange` 更新 `WizardData`。
- 生成按钮调用 `generate_base_preview` 时额外传递 `styleReferenceDataUrl`；不上传时传 `null`，保持现有 payload 形状的语义。
- 文案全部使用中文，明确说明“只参考画风，不复制人物内容”。

### 2. Tauri 命令与提示词

`generate_base_preview` 增加可选参数 `style_reference_data_url`。后端校验该值为有效图片 data URL，并继续用原始人物图选择色键。基础图核心流程将人物参考图和风格参考图作为独立可选输入传给 provider；不会把风格图保存到 manifest。

有风格图时，基础图提示词追加统一的风格参考契约：

> 图 1 是人物身份参考，图 2 是纯风格参考。保留图 1 的人物身份、面部特征、服装和配饰，只借鉴图 2 的线条、配色、材质、阴影、比例与整体气质；禁止复制图 2 的人物、服装、姿势、背景、构图、道具和文字。

没有风格图时不追加该契约，原有 `build_base_prompt` 输出保持兼容。

### 3. Provider 适配

#### SiliconFlow

只有选择了风格图时才切换到支持多图输入的参考图模型。默认使用 `Qwen/Qwen-Image-Edit-2509`：人物原图作为 `image`，风格图作为 `image2`，提示词明确两张图的职责；Qwen 编辑模型不发送不支持的 `image_size` 字段。无风格图时继续使用配置的基础模型和原来的文字生图 body。

如果配置的 SiliconFlow 参考模型不是支持 `image2` 的 Qwen 编辑模型，则返回明确错误，提示切换到 `Qwen/Qwen-Image-Edit-2509`，避免把风格图误当成普通主体图。

#### 万相

对 `wan2.6-image`、`wan2.7-image` 和 `wan2.7-image-pro`，将人物原图和风格图按顺序放入 `input.messages[].content`，并使用图号契约约束身份与风格职责；输出尺寸仍使用基础图的方形尺寸。无风格图时继续使用当前文字生图消息结构。

旧版 `wanx*` 模型没有可靠的多图纯风格入口，选择风格图时返回明确错误，提示切换到新版万相模型。

#### Local SD

Local SD 的基础阶段是 `txt2img`，没有标准的纯风格参考参数。选择风格图时返回明确错误并提示使用支持多图参考的云端模型；不改变现有 Local SD 的无风格路径。

### 4. 数据边界

```text
GenerateStep styleReferenceDataUrl
  -> WizardData.styleReferenceDataUrl
  -> generate_base_preview(styleReferenceDataUrl)
  -> generate_base(..., character_reference, style_reference)
  -> SiliconFlow / Wanxiang 多图请求

confirmed canonical base
  -> StateGenerationStep
  -> only canonical base reference for animation rows
```

`styleReferenceDataUrl` 不传给 `generate_state_row`，不加入 `GenerationRunManifest`，也不进入保存后的宠物数据。

## 测试策略

### TypeScript

- `GenerateStep` 能渲染风格图上传控件、预览和移除操作。
- 选择风格图后 `generate_base_preview` payload 包含 `styleReferenceDataUrl`；没有选择时为 `null`。
- 重新生成沿用同一张风格图，确认基础图时不会额外调用状态行命令。
- `WizardData` 初始值和创建器重置路径清空风格图。

### Rust

- SiliconFlow 无风格图时保持原有文字生图 body；有风格图时使用 Qwen `image` + `image2`，且不发送 `image_size`。
- 万相新版有风格图时生成包含两张图片和图号说明的消息；旧版模型拒绝风格图。
- Local SD 拒绝风格图，原有 `txt2img` body 不变。
- 基础图提示词只在有风格图时加入风格参考契约，且明确禁止复制风格图主体。
- data URL 校验拒绝空值、非图片类型、无效 base64 和超限内容。
- 运行现有 TypeScript/Vitest、Rust library tests、生产构建和 `git diff --check`。

## 验收标准

- 第 3 步可选上传一张风格参考图，并能看到预览、移除和中文说明。
- 选择风格图后，默认 SiliconFlow Qwen 或新版万相会收到人物图 + 风格图，并明确按“身份 / 画风”分工。
- 不选择风格图时，现有基础图生成流程完全不变。
- Local SD、旧版万相或不兼容的 SiliconFlow 参考模型不会静默忽略风格图，而是给出可操作的错误提示。
- 第 4 步和最终保存数据不包含风格参考图。

