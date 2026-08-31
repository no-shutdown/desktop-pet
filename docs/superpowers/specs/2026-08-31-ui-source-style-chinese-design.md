# 参考图风格中文界面设计

## 目标

将最近新增的参考图风格选择区域改为中文界面，避免用户在创建器中看到英文选项。

## 范围

只修改 `src/windows/Creator/steps/AnalyzeStep.tsx` 中的显示文案：

- `Source image style` → `参考图风格`
- `Realistic person photo` → `真实人物照片`
- `Convert to a cute 2D chibi character` → `转换为可爱的 2D Q 版形象`
- `Stylized artwork` → `卡通 / 插画作品`
- `Preserve the original art style` → `保留原始画风`

`realistic` 和 `stylized` 枚举值、回调参数、Rust manifest 字段及生成逻辑保持不变。品牌名、模型名和提示词不在本次范围内。

## 验证

为 AnalyzeStep 增加回归断言，确认风格选择区域显示上述中文文案且仍能选择两种风格；运行对应 Vitest 测试和生产构建。
