import { VISION_DESCRIBE_TEXT, VISION_SYSTEM_PROMPT } from './vision-prompt';

interface ClaudeMessage {
  role: 'user';
  content: Array<
    | { type: 'image'; source: { type: 'base64'; media_type: string; data: string } }
    | { type: 'text'; text: string }
  >;
}

export function buildAnalysisMessages(imageDataUrl: string): ClaudeMessage[] {
  const [header, data] = imageDataUrl.split(',');
  const mediaType = header.replace('data:', '').replace(';base64', '');

  return [
    {
      role: 'user',
      content: [
        {
          type: 'image',
          source: { type: 'base64', media_type: mediaType, data },
        },
        {
          type: 'text',
          text: VISION_DESCRIBE_TEXT,
        },
      ],
    },
  ];
}

export async function analyzePhoto(imageDataUrl: string, apiKey: string): Promise<string> {
  if (!apiKey) throw new Error('API key required');

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
      system: VISION_SYSTEM_PROMPT,
      messages: buildAnalysisMessages(imageDataUrl),
    }),
  });

  if (!response.ok) {
    throw new Error(`${response.status}: Claude Vision API error`);
  }

  const responseData = await response.json();
  return responseData.content[0].text as string;
}
