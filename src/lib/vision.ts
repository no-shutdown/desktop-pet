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
        'https://api.deepseek.com',
        settings.visionModel,
        { thinking: { type: 'disabled' } },
      );
    case 'kimi':
      return analyzeOpenAICompat(
        imageDataUrl,
        settings.visionApiKey,
        'https://api.moonshot.cn/v1',
        settings.visionModel,
        kimiVisionOptions(settings.visionModel),
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
  if (!response.ok) {
    throw new Error(`Anthropic API error: ${response.status}${await apiErrorDetail(response)}`);
  }
  const data = await response.json();
  return data.content[0].text as string;
}

async function analyzeOpenAICompat(
  imageDataUrl: string,
  apiKey: string,
  baseUrl: string,
  model: string,
  options: OpenAICompatOptions = {},
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
      ...options,
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
  if (!response.ok) {
    throw new Error(`${model} API error: ${response.status}${await apiErrorDetail(response)}`);
  }
  const data = await response.json() as {
    choices?: Array<{ message?: { content?: unknown } }>;
  };
  return extractAssistantText(data, model);
}

type OpenAICompatOptions = {
  thinking?: { type: 'enabled' | 'disabled' };
  reasoning_effort?: 'low' | 'high' | 'max';
  max_tokens?: number;
};

function kimiVisionOptions(model: string): OpenAICompatOptions {
  if (model === 'kimi-k3') {
    return { max_tokens: 512, reasoning_effort: 'low' };
  }
  return { thinking: { type: 'disabled' } };
}

function extractAssistantText(
  data: { choices?: Array<{ message?: { content?: unknown } }> },
  model: string,
): string {
  const content = data.choices?.[0]?.message?.content;
  const text = typeof content === 'string'
    ? content
    : Array.isArray(content)
      ? content
        .map((part) => {
          if (typeof part !== 'object' || part === null || !('text' in part)) return '';
          return typeof part.text === 'string' ? part.text : '';
        })
        .join('')
      : '';

  if (!text.trim()) {
    throw new Error(`${model} API returned an empty description`);
  }
  return text.trim();
}

async function apiErrorDetail(response: Response): Promise<string> {
  try {
    const data = await response.json() as {
      error?: { message?: unknown };
      message?: unknown;
    };
    const message = typeof data.error?.message === 'string'
      ? data.error.message
      : typeof data.message === 'string'
        ? data.message
        : '';
    return message.trim() ? `: ${message.trim()}` : '';
  } catch {
    return '';
  }
}
