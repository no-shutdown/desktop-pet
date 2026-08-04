interface ClaudeMessage {
  role: 'user';
  content: Array<
    | { type: 'image'; source: { type: 'base64'; media_type: string; data: string } }
    | { type: 'text'; text: string }
  >;
}

const SYSTEM_PROMPT =
  'You are a character design assistant. Analyze the reference photo and describe the character in detail for generating a Q-version chibi desktop pet. Focus on: hair color and style, face shape, skin tone, eye color, clothing colors and style, notable accessories. Output a single comma-separated description suitable as a Stable Diffusion prompt. Be concise (under 80 words).';

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
          text: 'Describe this character for a chibi desktop pet prompt.',
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
      system: SYSTEM_PROMPT,
      messages: buildAnalysisMessages(imageDataUrl),
    }),
  });

  if (!response.ok) {
    throw new Error(`${response.status}: Claude Vision API error`);
  }

  const responseData = await response.json();
  return responseData.content[0].text as string;
}
