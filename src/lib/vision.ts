import { buildAnalysisMessages } from './claude-vision';
import type { AppSettings } from './settings';

const SYSTEM_PROMPT =
  'You are a character design assistant. Analyze the reference photo and describe the character in detail for generating a Q-version chibi desktop pet. Focus on: hair color and style, face shape, skin tone, eye color, clothing colors and style, notable accessories. Output a single comma-separated description suitable as a Stable Diffusion prompt. Be concise (under 80 words).';

const DESCRIBE_TEXT = 'Describe this character for a chibi desktop pet prompt.';

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
      return analyzeWithAnthropic(imageDataUrl, settings.visionApiKey);
    case 'deepseek':
      return analyzeOpenAICompat(
        imageDataUrl,
        settings.visionApiKey,
        'https://api.deepseek.com/v1',
        'deepseek-vl2',
      );
    case 'kimi':
      return analyzeOpenAICompat(
        imageDataUrl,
        settings.visionApiKey,
        'https://api.moonshot.cn/v1',
        'moonshot-v1-8k-vision-preview',
      );
    default:
      throw new Error(`Unknown vision provider: ${settings.visionProvider}`);
  }
}

async function analyzeWithAnthropic(imageDataUrl: string, apiKey: string): Promise<string> {
  const response = await fetch('https://api.anthropic.com/v1/messages', {
    method: 'POST',
    headers: {
      'x-api-key': apiKey,
      'anthropic-version': '2023-06-01',
      'content-type': 'application/json',
    },
    body: JSON.stringify({
      model: 'claude-opus-4-7',
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
