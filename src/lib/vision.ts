import { buildAnalysisMessages } from './claude-vision';
import type { AppSettings } from './settings';

const SYSTEM_PROMPT =
  'Analyze the reference image for a desktop-pet generator. First identify the source medium/style as either a realistic human photo (photorealistic) or stylized artwork (cartoon, anime, illustration, or pixel art). Describe the character\'s recognizable features and also the source style\'s line quality, proportions, palette, shading, and texture. Preserve the source medium and style in the description; do not convert existing stylized artwork into generic Q-version wording. Output one concise comma-separated character description under 80 words; the caller separately chooses whether a realistic photo should be transformed into a cute 2D chibi illustration.';

const DESCRIBE_TEXT =
  'Describe this character faithfully, preserving its source medium and style in the description for a desktop-pet prompt.';

export async function analyzePhotoWithSettings(
  imageDataUrl: string,
  settings: AppSettings,
): Promise<string> {
  if (settings.visionProvider === 'skip') {
    throw new Error('Vision analysis is disabled in settings.');
  }
  if (!settings.visionApiKey) {
    throw new Error('No API key configured. Open Settings to add one.');
  }

  switch (settings.visionProvider) {
    case 'anthropic':
      return analyzeWithAnthropic(imageDataUrl, settings.visionApiKey, settings.visionModel);
    case 'deepseek':
      return analyzeOpenAICompat(
        imageDataUrl,
        settings.visionApiKey,
        'https://api.deepseek.com/v1',
        settings.visionModel,
      );
    case 'kimi':
      return analyzeOpenAICompat(
        imageDataUrl,
        settings.visionApiKey,
        'https://api.moonshot.cn/v1',
        settings.visionModel,
      );
    default:
      throw new Error(`Unknown vision provider: ${settings.visionProvider}`);
  }
}

async function analyzeWithAnthropic(imageDataUrl: string, apiKey: string, model: string): Promise<string> {
  const response = await fetch('https://api.anthropic.com/v1/messages', {
    method: 'POST',
    headers: {
      'x-api-key': apiKey,
      'anthropic-version': '2023-06-01',
      'content-type': 'application/json',
    },
    body: JSON.stringify({
      model,
      max_tokens: 256,
      system: SYSTEM_PROMPT,
      messages: buildAnalysisMessages(imageDataUrl),
    }),
  });
  if (!response.ok) throw new Error(`Anthropic API error: ${response.status}`);
  const data = await response.json();
  return data.content[0].text as string;
}

async function analyzeOpenAICompat(
  imageDataUrl: string,
  apiKey: string,
  baseUrl: string,
  model: string,
): Promise<string> {
  const response = await fetch(`${baseUrl}/chat/completions`, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${apiKey}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      model,
      max_tokens: 256,
      messages: [
        {
          role: 'user',
          content: [
            { type: 'image_url', image_url: { url: imageDataUrl } },
            { type: 'text', text: `${SYSTEM_PROMPT}\n\n${DESCRIBE_TEXT}` },
          ],
        },
      ],
    }),
  });
  if (!response.ok) throw new Error(`${model} API error: ${response.status}`);
  const data = await response.json();
  return data.choices[0].message.content as string;
}
