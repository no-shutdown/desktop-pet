import { describe, it, expect, vi, beforeEach } from 'vitest';
import { analyzePhotoWithSettings } from '../vision';
import { DEFAULT_SETTINGS, type AppSettings } from '../settings';

const mockFetch = vi.fn();
vi.stubGlobal('fetch', mockFetch);

import { analyzePhoto, buildAnalysisMessages } from '../claude-vision';

type RequestBody = {
  system?: string;
  messages: Array<{ content: Array<{ type: string; text?: string }> }>;
};

function getRequestBody(): RequestBody {
  const request = mockFetch.mock.calls[0][1] as { body: string };
  return JSON.parse(request.body) as RequestBody;
}

function settingsFor(visionProvider: AppSettings['visionProvider']): AppSettings {
  return {
    ...DEFAULT_SETTINGS,
    visionProvider,
    visionApiKey: 'vision-test-key',
    visionModel: 'vision-test-model',
  };
}

function messageText(body: RequestBody): string {
  return body.messages[0].content.find((item) => item.type === 'text')?.text ?? '';
}

function expectSourceStyleContract(instruction: string): void {
  const normalized = instruction.toLowerCase();
  expect(normalized).toContain('source medium/style');
  expect(normalized).toContain('preserve');
  expect(normalized).toContain('photorealistic');
  expect(normalized).toContain('faithfully');
  expect(normalized).toContain('do not convert existing stylized artwork into generic q-version wording');
}

describe('claude-vision', () => {
  beforeEach(() => mockFetch.mockReset());

  it('buildAnalysisMessages returns array with image and text content', () => {
    const msgs = buildAnalysisMessages('data:image/jpeg;base64,abc123');
    expect(msgs).toHaveLength(1);
    expect(msgs[0].role).toBe('user');
    const content = msgs[0].content as { type: string }[];
    expect(content.some((c) => c.type === 'image')).toBe(true);
    expect(content.some((c) => c.type === 'text')).toBe(true);
  });

  it('asks vision analysis to identify and preserve the input medium', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ content: [{ type: 'text', text: 'stylized cartoon, orange fox' }] }),
    });

    await analyzePhoto('data:image/jpeg;base64,abc123', 'sk-ant-test');
    const body = getRequestBody();
    const systemInstruction = body.system?.toLowerCase() ?? '';
    const userInstruction = messageText(body).toLowerCase();

    expect(systemInstruction).toContain('source medium/style');
    expect(systemInstruction).toContain('preserve');
    expect(systemInstruction).toContain('photorealistic');
    expect(systemInstruction).toContain('faithfully');
    expect(systemInstruction).toContain(
      'do not convert existing stylized artwork into generic q-version wording'
    );
    expect(userInstruction).toContain('faithfully');
    expect(userInstruction).toContain('source medium and style');
  });

  it('analyzePhotoWithSettings sends the source-style contract to Anthropic', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ content: [{ type: 'text', text: 'source-style description' }] }),
    });

    await analyzePhotoWithSettings('data:image/jpeg;base64,abc123', settingsFor('anthropic'));
    const body = getRequestBody();

    expectSourceStyleContract(body.system ?? '');
    expect(messageText(body).toLowerCase()).toContain('source medium and style');
  });

  it.each([
    {
      name: 'DeepSeek',
      provider: 'deepseek' as const,
      endpoint: 'https://api.deepseek.com/v1/chat/completions',
    },
    {
      name: 'Kimi',
      provider: 'kimi' as const,
      endpoint: 'https://api.moonshot.cn/v1/chat/completions',
    },
  ])(
    'analyzePhotoWithSettings sends the source-style contract to $name',
    async ({ provider, endpoint }) => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ choices: [{ message: { content: 'source-style description' } }] }),
      });

      await analyzePhotoWithSettings('data:image/jpeg;base64,abc123', settingsFor(provider));
      const body = getRequestBody();

      expect(mockFetch).toHaveBeenCalledWith(endpoint, expect.anything());
      expectSourceStyleContract(messageText(body));
    }
  );

  it('analyzePhoto calls Anthropic API with correct headers', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        content: [{ type: 'text', text: 'anime chibi girl, black hair' }],
      }),
    });

    await analyzePhoto('data:image/jpeg;base64,abc123', 'sk-ant-test');

    expect(mockFetch).toHaveBeenCalledWith(
      'https://api.anthropic.com/v1/messages',
      expect.objectContaining({
        method: 'POST',
        headers: expect.objectContaining({
          'x-api-key': 'sk-ant-test',
          'anthropic-version': '2023-06-01',
        }),
      })
    );
  });

  it('analyzePhoto returns character description string', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        content: [{ type: 'text', text: 'anime chibi girl, black hair, red dress' }],
      }),
    });

    const result = await analyzePhoto('data:image/jpeg;base64,abc123', 'sk-ant-test');
    expect(result).toBe('anime chibi girl, black hair, red dress');
  });

  it('analyzePhoto throws when API key is empty', async () => {
    await expect(analyzePhoto('data:image/jpeg;base64,abc', '')).rejects.toThrow(
      'API key required'
    );
  });

  it('analyzePhoto throws when API returns error', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 401,
      json: async () => ({ error: { message: 'invalid key' } }),
    });

    await expect(analyzePhoto('data:image/jpeg;base64,abc', 'bad-key')).rejects.toThrow(
      '401'
    );
  });
});
