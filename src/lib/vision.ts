import { buildAnalysisMessages } from './claude-vision';
import type { AppSettings } from './settings';
import { VISION_DESCRIBE_TEXT, VISION_SYSTEM_PROMPT } from './vision-prompt';

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
      system: VISION_SYSTEM_PROMPT,
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
            { type: 'text', text: `${VISION_SYSTEM_PROMPT}\n\n${VISION_DESCRIBE_TEXT}` },
          ],
        },
      ],
    }),
  });
  if (!response.ok) throw new Error(`${model} API error: ${response.status}`);
  const data = await response.json();
  return data.choices[0].message.content as string;
}
