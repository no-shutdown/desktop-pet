# 基础图风格参考图 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在创建器基础图步骤支持一张可选的纯风格参考图，并让支持多图输入的基础图 provider 同时收到人物身份图与风格图，而不影响无风格图流程和后续状态行。

**Architecture:** React 在 WizardData 中保存风格图 data URL，并由 GenerateStep 控制上传、预览、移除和基础图 payload。Tauri generate_base_preview 验证并转发人物图与风格图；Rust provider 只在存在风格图时进入明确的多图分支，SiliconFlow 使用 Qwen image/image2，新版万相使用带图号的消息，旧版万相和 Local SD 返回可操作错误。风格参考图不进入 manifest、状态行或最终宠物数据。

**Tech Stack:** React 19 + TypeScript + Vitest + Testing Library；Tauri 2 command；Rust + Tokio + Reqwest + serde_json；PowerShell 验证命令。

---

## 文件地图

- Modify: src/windows/Creator/steps/types.ts — 向导内的可选风格图状态和初始值。
- Modify: src/windows/Creator/index.tsx — 连接风格图状态，并在放弃/重置生成时清空它。
- Modify: src/windows/Creator/steps/GenerateStep.tsx — 中文上传控件、预览/移除和基础图 payload。
- Test: src/windows/Creator/steps/__tests__/GenerateStep.test.tsx — 上传、移除、payload 和重试保留。
- Test: src/windows/Creator/steps/__tests__/types.test.ts — 初始状态不带风格图。
- Modify: src-tauri/src/commands/generation/prompts.rs — 双图身份/风格图号契约。
- Modify: src-tauri/src/commands/generation/providers.rs — SiliconFlow Qwen 双图 body、万相新版双图 body 和能力拒绝。
- Modify: src-tauri/src/commands/generation/mod.rs — data URL 校验、基础图核心透传和 Tauri command 参数。

## Task 1: 先为前端状态和上传交互写失败测试

**Files:**
- Modify: src/windows/Creator/steps/__tests__/GenerateStep.test.tsx
- Create: src/windows/Creator/steps/__tests__/types.test.ts

- [ ] **Step 1: Define the new UI and payload assertions**

在 GenerateStep.test.tsx 的默认 props 中加入 styleReferenceDataUrl: null 和 onStyleReferenceChange: vi.fn()。新增断言：

~~~tsx
it('shows an optional style-reference upload control', () => {
  render(<GenerateStep {...defaultProps} runId="run-1" />);

  expect(screen.getByText('风格参考图（可选）')).toBeTruthy();
  expect(screen.getByTestId('style-reference-file-input')).toHaveAttribute(
    'accept', 'image/jpeg,image/png,image/webp',
  );
  expect(screen.getByText('只参考画风，不复制图片中的人物内容')).toBeTruthy();
});

it('reads, previews, and removes a style reference image', () => {
  render(<GenerateStep {...defaultProps} runId="run-1" />);
  const input = screen.getByTestId('style-reference-file-input') as HTMLInputElement;
  Object.defineProperty(input, 'files', {
    value: [new File(['fake'], 'style.png', { type: 'image/png' })],
  });
  let reader: { onload: ((event: ProgressEvent) => void) | null; result: string } | null = null;
  class MockFileReader {
    onload: ((event: ProgressEvent) => void) | null = null;
    result = 'data:image/png;base64,STYLE';
    readAsDataURL = vi.fn();
    constructor() { reader = this; }
  }
  vi.stubGlobal('FileReader', MockFileReader);

  fireEvent.change(input);
  reader?.onload?.({ target: reader } as unknown as ProgressEvent);
  expect(screen.getByAltText('风格参考图预览')).toHaveAttribute(
    'src', 'data:image/png;base64,STYLE',
  );
  expect(defaultProps.onStyleReferenceChange).toHaveBeenLastCalledWith(
    'data:image/png;base64,STYLE',
  );

  fireEvent.click(screen.getByRole('button', { name: '移除风格参考图' }));
  expect(screen.queryByAltText('风格参考图预览')).toBeNull();
  expect(defaultProps.onStyleReferenceChange).toHaveBeenLastCalledWith(null);
  vi.unstubAllGlobals();
});
~~~

把现有基础图请求断言扩展为 styleReferenceDataUrl: null，并增加一次已读取风格图后请求值为 data:image/png;base64,STYLE 的断言。types.test.ts 使用：

~~~tsx
import { describe, expect, it } from 'vitest';
import { INITIAL_WIZARD_DATA } from '../types';

describe('WizardData', () => {
  it('starts without a base-image style reference', () => {
    expect(INITIAL_WIZARD_DATA.styleReferenceDataUrl).toBeNull();
  });
});
~~~

- [ ] **Step 2: Run focused tests to verify RED**

~~~powershell
npm exec vitest run src/windows/Creator/steps/__tests__/GenerateStep.test.tsx src/windows/Creator/steps/__tests__/types.test.ts
~~~

Expected: FAIL because the field, input and callback do not exist yet.

## Task 2: Implement前端风格图上传、状态同步和基础图 payload

**Files:**
- Modify: src/windows/Creator/steps/types.ts
- Modify: src/windows/Creator/index.tsx
- Modify: src/windows/Creator/steps/GenerateStep.tsx
- Test: src/windows/Creator/steps/__tests__/GenerateStep.test.tsx

- [ ] **Step 1: Add the wizard field and reset behavior**

在 WizardData 增加：

~~~ts
styleReferenceDataUrl: string | null;
~~~

并在 INITIAL_WIZARD_DATA 设置 styleReferenceDataUrl: null。在 abandonGeneration() 的 patch 中清空该字段；整体替换为 INITIAL_WIZARD_DATA 的 reset/新建 AI 路径保持不变。从状态行返回基础图时不要清空。

为 GenerateStepProps 增加：

~~~ts
styleReferenceDataUrl?: string | null;
onStyleReferenceChange?: (dataUrl: string | null) => void;
~~~

在 CreatorWindow 传入：

~~~tsx
styleReferenceDataUrl={data.styleReferenceDataUrl}
onStyleReferenceChange={(styleReferenceDataUrl) => updateData({ styleReferenceDataUrl })}
~~~

- [ ] **Step 2: Add upload state and Chinese UI**

在 GenerateStep 用本地 state 初始化 prop，并同步外部值：

~~~tsx
const [styleReference, setStyleReference] = useState<string | null>(styleReferenceDataUrl ?? null);

useEffect(() => {
  setStyleReference(styleReferenceDataUrl ?? null);
}, [styleReferenceDataUrl]);

function updateStyleReference(dataUrl: string | null) {
  setStyleReference(dataUrl);
  onStyleReferenceChange?.(dataUrl);
}
~~~

文件变化时只接受 image/jpeg、image/png、image/webp，通过 FileReader.readAsDataURL 传给 updateStyleReference。渲染隐藏的 data-testid="style-reference-file-input"，有图时显示 alt="风格参考图预览" 与“移除风格参考图”；无图时显示“上传一张参考画风的图片（可选）”和“只参考画风，不复制图片中的人物内容”。移除时清空 input value，允许再次选择同一文件。

- [ ] **Step 3: Add only the base command field**

在 handleGenerate 的 invoke('generate_base_preview', { ... }) 中加入：

~~~tsx
styleReferenceDataUrl: styleReference || null,
~~~

保持 referenceDataUrl 代表原始人物图；不要修改 StateGenerationStep、generate_state_row 或状态行 payload。

- [ ] **Step 4: Run frontend tests to verify GREEN**

~~~powershell
npm exec vitest run src/windows/Creator/steps/__tests__/GenerateStep.test.tsx src/windows/Creator/steps/__tests__/types.test.ts
~~~

Expected: focused tests pass；重试两次的 payload 相同，且不会调用 generate_state_row 或 generate_and_assemble。

## Task 3: 先为提示词和 provider 请求体写失败测试

**Files:**
- Modify: src-tauri/src/commands/generation/prompts.rs
- Modify: src-tauri/src/commands/generation/providers.rs

- [ ] **Step 1: Test the prompt role contract**

导入即将新增的 build_base_prompt_with_style_reference，增加：

~~~rust
#[test]
fn base_prompt_assigns_identity_to_image_one_and_style_to_image_two() {
    let prompt = build_base_prompt_with_style_reference(
        "a character", SourceStyle::Realistic, "#FF00FF", "magenta", true,
    );

    assert!(prompt.contains("image 1 is the original character identity reference"));
    assert!(prompt.contains("image 2 is a pure style reference"));
    assert!(prompt.contains("do not copy image 2's subject"));
}
~~~

同时断言传 false 时不包含 image 2 is a pure style reference。

- [ ] **Step 2: Test exact provider bodies**

新增测试应要求：

~~~rust
let body = siliconflow_base_body_with_references(
    "Qwen/Qwen-Image-Edit-2509", "base prompt",
    "data:image/jpeg;base64,CHARACTER", "data:image/png;base64,STYLE",
).unwrap();
assert_eq!(body["image"], "data:image/jpeg;base64,CHARACTER");
assert_eq!(body["image2"], "data:image/png;base64,STYLE");
assert!(body.get("image_size").is_none());

let body = wanxiang_base_body_with_references(
    "wan2.7-image", "base prompt",
    "data:image/jpeg;base64,CHARACTER", "data:image/png;base64,STYLE",
).unwrap();
let content = &body["input"]["messages"][0]["content"];
assert_eq!(content[0]["image"], "data:image/jpeg;base64,CHARACTER");
assert_eq!(content[1]["image"], "data:image/png;base64,STYLE");
assert_eq!(content[2]["text"], "base prompt");
~~~

另测 Kwai-Kolors/Kolors、wanx2.1-t2i-turbo 的双图 helper 返回包含切换到支持模型建议的错误；现有无风格图 body 测试保持原样。

- [ ] **Step 3: Run Rust tests to verify RED**

~~~powershell
cargo test --manifest-path src-tauri/Cargo.toml generation::prompts::tests generation::providers::tests
~~~

Expected: FAIL because the new helpers 和契约尚未实现。

## Task 4: Implement prompt and provider capability branches

**Files:**
- Modify: src-tauri/src/commands/generation/prompts.rs
- Modify: src-tauri/src/commands/generation/providers.rs

- [ ] **Step 1: Add a backward-compatible prompt builder**

保留原四参数 build_base_prompt，让它调用新增函数并传 false：

~~~rust
pub fn build_base_prompt_with_style_reference(
    base_description: &str,
    source_style: SourceStyle,
    chroma_hex: &str,
    chroma_name: &str,
    has_style_reference: bool,
) -> String
~~~

新增常量：

~~~rust
const STYLE_REFERENCE_CONTRACT: &str =
    "STYLE REFERENCE CONTRACT: image 1 is the original character identity reference and image 2 is a pure style reference. Preserve image 1's identity, face, clothing, and accessories; borrow only image 2's line quality, palette, materials, shading, proportions, and overall charm. Do not copy image 2's subject, clothing, pose, background, composition, props, or text.";
~~~

只有 has_style_reference 为 true 时将其插入最终 prompt。

- [ ] **Step 2: Add provider body builders without changing no-style bodies**

新增并返回 Result<Value, String>：

~~~rust
pub fn siliconflow_base_body_with_references(
    model: &str, prompt: &str,
    character_image_data_url: &str, style_image_data_url: &str,
) -> Result<Value, String> {
    if model.trim() != "Qwen/Qwen-Image-Edit-2509" {
        return Err("SiliconFlow 风格参考需要切换到 Qwen/Qwen-Image-Edit-2509 参考模型".to_string());
    }
    Ok(serde_json::json!({
        "model": model, "prompt": prompt,
        "image": character_image_data_url, "image2": style_image_data_url,
        "num_inference_steps": 20,
    }))
}

pub fn wanxiang_base_body_with_references(
    model: &str, prompt: &str,
    character_image_data_url: &str, style_image_data_url: &str,
) -> Result<Value, String> {
    if !is_new_wan_model(model) {
        return Err("万相风格参考需要切换到 wan2.6 或 wan2.7 新版模型".to_string());
    }
    Ok(serde_json::json!({
        "model": model,
        "input": { "messages": [{ "role": "user", "content": [
            {"image": character_image_data_url},
            {"image": style_image_data_url},
            {"text": prompt}
        ]}]},
        "parameters": { "size": "1024*1024", "n": 1 }
    }))
}
~~~

Local SD 的拒绝错误放在 generate_base 的 provider 分支；不要给无风格图的任何 body 添加新字段。

- [ ] **Step 3: Run focused Rust tests to verify GREEN**

~~~powershell
cargo test --manifest-path src-tauri/Cargo.toml generation::prompts::tests generation::providers::tests
~~~

Expected: new prompt/provider tests 和现有 contract tests 全部 PASS。

## Task 5: 先为 Tauri 透传、校验和不支持 provider 写失败测试

**Files:**
- Modify: src-tauri/src/commands/generation/mod.rs

- [ ] **Step 1: Extend base-core closure assertions**

把 generate_base_preview_core_at 的测试闭包统一改为四个输入参数：

~~~rust
move |
    _config: &ProviderConfig,
    prompt: &str,
    character_reference: Option<&str>,
    style_reference: Option<&str>,
| {
    assert_eq!(character_reference, Some("data:image/jpeg;base64,CHARACTER"));
    assert_eq!(style_reference, Some("data:image/png;base64,STYLE"));
    assert!(prompt.contains("image 2 is a pure style reference"));
    let provider_image = provider_image.clone();
    async move { Ok(provider_image) }
}
~~~

现有无风格图测试断言两个 reference 都为 None，保护旧路径。

- [ ] **Step 2: Add validation and capability tests**

增加对如下输入全部返回错误的测试：

~~~rust
assert!(validate_style_reference_data_url("https://example.com/style.png").is_err());
assert!(validate_style_reference_data_url("data:text/plain;base64,QQ==").is_err());
assert!(validate_style_reference_data_url("data:image/png;base64,not-base64").is_err());
assert!(validate_style_reference_data_url("data:image/png;base64,").is_err());
~~~

另用 generate_base 的已配置 localsd、旧版万相和非 Qwen SiliconFlow 配置，断言有双图参数时在网络调用前返回包含“风格参考”或切换模型建议的错误。

- [ ] **Step 3: Run Rust tests to verify RED**

~~~powershell
cargo test --manifest-path src-tauri/Cargo.toml generation::base_generation_core_tests generation::providers::tests
~~~

Expected: FAIL because核心闭包签名、验证 helper 和 style dispatch 尚未实现。

## Task 6: Implement Tauri command data flow and provider dispatch

**Files:**
- Modify: src-tauri/src/commands/generation/mod.rs
- Test: src-tauri/src/commands/generation/mod.rs

- [ ] **Step 1: Add bounded data URL validation**

新增 MAX_REFERENCE_IMAGE_BYTES = 16 * 1024 * 1024 和 validate_style_reference_data_url。实现必须：trim 输入；要求 data:image/...;base64,；拒绝空 payload、无效 base64、空解码结果和超过 16 MiB 的 decoded bytes；成功时返回规范化字符串。风格图存在但 reference_data_url 为空时，command 返回“风格参考需要原始人物参考图”。

- [ ] **Step 2: Thread both references through the core**

将 core signature 改为：

~~~rust
async fn generate_base_preview_core_at<F, Fut>(
    app_data_dir: &Path, run_id: String, base_prompt: String,
    provider_config: ProviderConfig,
    requested_source_style: Option<SourceStyle>, selected_key: ChromaKey,
    character_reference_data_url: Option<String>,
    style_reference_data_url: Option<String>, provider_call: F,
) -> Result<RgbaImage, String>
where
    F: FnOnce(&ProviderConfig, &str, Option<&str>, Option<&str>) -> Fut,
    Fut: Future<Output = Result<Vec<u8>, String>>,
~~~

用 build_base_prompt_with_style_reference(..., style_reference_data_url.is_some())，并将两个 URL 的 as_deref() 传给 closure。manifest 只继续保存 prompt、source style 和生成状态，不新增风格图字段。

- [ ] **Step 3: Extend only generate_base_preview**

在 command 参数末尾增加：

~~~rust
style_reference_data_url: Option<String>,
~~~

先校验/规范化风格图，再用原人物图完成现有 chroma-key 选择，然后在 closure 中调用：

~~~rust
generate_base(
    &config, &prompt,
    character_reference.as_deref(),
    style_reference.as_deref(),
).await
~~~

不要给 generate_state_row 增加参数。

- [ ] **Step 4: Add style dispatch to generate_base**

签名改为：

~~~rust
pub async fn generate_base(
    config: &ProviderConfig, prompt: &str,
    character_reference_data_url: Option<&str>,
    style_reference_data_url: Option<&str>,
) -> Result<Vec<u8>, String>
~~~

有 style reference 时要求 character reference 存在并分支：SiliconFlow 调用 siliconflow_base_body_with_references(&config.reference_model, ...)；万相调用 wanxiang_base_body_with_references(&config.base_model, ...) 和新版异步 endpoint；Local SD 返回中文不支持错误；未知 provider 保持现有错误。没有 style reference 时完整保留现有三个无风格分支。

- [ ] **Step 5: Run focused Rust tests to verify GREEN**

~~~powershell
cargo test --manifest-path src-tauri/Cargo.toml generation::base_generation_core_tests generation::providers::tests generation::source_style_parser_tests
~~~

Expected: all focused tests PASS，且不会发起真实网络请求。

## Task 7: 集成回归和最终验证

**Files:**
- No new production files; inspect all modified files above.

- [ ] **Step 1: Verify the data boundary**

~~~powershell
rg -n "styleReferenceDataUrl|style_reference_data_url|generate_state_row|generate_base_preview" src src-tauri/src/commands/generation
~~~

Expected: style field 只出现在向导和基础图链路；generate_state_row 没有风格参考参数或 payload。

- [ ] **Step 2: Run all TypeScript tests**

~~~powershell
npm test -- --run
~~~

Expected: Vitest exits 0；已有的非致命 React act/canvas warnings 可以保留。

- [ ] **Step 3: Run Rust library tests**

~~~powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib
~~~

Expected: 所有 Rust unit tests PASS。

- [ ] **Step 4: Run production build and whitespace check**

~~~powershell
npm run build
git diff --check
git status --short --branch
~~~

Expected: tsc && vite build exits 0；无 whitespace error；template.webp 仍保持 untracked/unstaged，不执行 reset、checkout 或删除命令。
