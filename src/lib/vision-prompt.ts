export const VISION_SYSTEM_PROMPT =
  'Analyze the reference image for a desktop-pet generator. First identify the source medium/style as either a realistic human photo (photorealistic) or stylized artwork (cartoon, anime, illustration, or pixel art). Describe the character faithfully, including recognizable features and the source style\'s line quality, proportions, palette, shading, and texture. Preserve the source medium and style in the description; do not convert existing stylized artwork into generic Q-version wording. Output one concise comma-separated character description under 80 words; the caller separately chooses whether a realistic photo should be transformed into a cute 2D chibi illustration.';

export const VISION_DESCRIBE_TEXT =
  'Describe this character faithfully, preserving its source medium and style in the description for a desktop-pet prompt.';
