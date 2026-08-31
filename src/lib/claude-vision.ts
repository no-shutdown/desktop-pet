interface ClaudeMessage {
  role: 'user';
  content: Array<
    | { type: 'image'; source: { type: 'base64'; media_type: string; data: string } }
    | { type: 'text'; text: string }
  >;
}

const SYSTEM_PROMPT =
  'Analyze the reference image for a desktop-pet generator. First identify the source medium/style as either a realistic human photo (photorealistic) or stylized artwork (cartoon, anime, illustration, or pixel art). Describe the character\'s recognizable features and also the source style\'s line quality, proportions, palette, shading, and texture. Preserve the source medium and style in the description; do not convert existing stylized artwork into generic Q-version wording. Output one concise comma-separated character description under 80 words; the caller separately chooses whether a realistic photo should be transformed into a cute 2D chibi illustration.';

const DESCRIBE_TEXT =
  'Describe this character faithfully, preserving its source medium and style in the description for a desktop-pet prompt.';

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
          text: DESCRIBE_TEXT,
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
