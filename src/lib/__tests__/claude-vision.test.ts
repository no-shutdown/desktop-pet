import { describe, it, expect, vi, beforeEach } from 'vitest';
import { analyzePhotoWithSettings } from '../vision';
import { DEFAULT_SETTINGS, type AppSettings } from '../settings';

const mockFetch = vi.fn();
vi.stubGlobal('fetch', mockFetch);

import { analyzePhoto, buildAnalysisMessages } from '../claude-vision';

type RequestBody = {
  model?: string;
  max_tokens?: number;
  thinking?: { type?: string };
  reasoning_effort?: string;
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

function expectChineseCharacterOnlyContract(instruction: string): void {
  expect(instruction).toContain('中文');
  expect(instruction).toContain('第一印象');
  expect(instruction).toContain('脸型');
  expect(instruction).toContain('发型');
  expect(instruction).toContain('不要描述背景');
  expect(instruction).toContain('不要描述动作');
  expect(instruction).toContain('镜头');
  expect(instruction).toContain('光线');
  expect(instruction).toContain('景深');
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

  it('asks vision analysis for a Chinese character-only first impression', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ content: [{ type: 'text', text: '橙色短毛的小狐狸角色' }] }),
    });

    await analyzePhoto('data:image/jpeg;base64,abc123', 'sk-ant-test');
    const body = getRequestBody();
    expectChineseCharacterOnlyContract(body.system ?? '');
    expectChineseCharacterOnlyContract(messageText(body));
  });

  it('analyzePhotoWithSettings sends the Chinese character-only contract to Anthropic', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ content: [{ type: 'text', text: '圆脸黑色长发的年轻女性' }] }),
    });

    await analyzePhotoWithSettings('data:image/jpeg;base64,abc123', settingsFor('anthropic'));
    const body = getRequestBody();

    expectChineseCharacterOnlyContract(body.system ?? '');
    expectChineseCharacterOnlyContract(messageText(body));
  });

  it.each([
    {
      name: 'DeepSeek',
      provider: 'deepseek' as const,
      endpoint: 'https://api.deepseek.com/chat/completions',
    },
    {
      name: 'Kimi',
      provider: 'kimi' as const,
      endpoint: 'https://api.moonshot.cn/v1/chat/completions',
    },
  ])(
    'analyzePhotoWithSettings sends the Chinese character-only contract to $name',
    async ({ provider, endpoint }) => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ choices: [{ message: { content: '圆脸黑色长发的年轻女性' } }] }),
      });

      await analyzePhotoWithSettings('data:image/jpeg;base64,abc123', settingsFor(provider));
      const body = getRequestBody();

      expect(mockFetch).toHaveBeenCalledWith(endpoint, expect.anything());
      expectChineseCharacterOnlyContract(messageText(body));
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

  it('includes OpenAI-compatible API error details', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 404,
      json: async () => ({ error: { message: 'model not found for this account' } }),
    });

    await expect(
      analyzePhotoWithSettings('data:image/jpeg;base64,abc', settingsFor('kimi'))
    ).rejects.toThrow('model not found for this account');
  });

  it.each(['deepseek', 'kimi'] as const)('disables thinking for short %s vision descriptions', async (provider) => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ choices: [{ message: { content: 'black-haired chibi character' } }] }),
    });

    await analyzePhotoWithSettings('data:image/jpeg;base64,abc', {
      ...settingsFor(provider),
      visionModel: provider === 'kimi' ? 'kimi-k2.6' : 'deepseek-v4-flash-vision-exp',
    });

    const body = getRequestBody();
    expect(body.model).toBe(provider === 'kimi' ? 'kimi-k2.6' : 'deepseek-v4-flash-vision-exp');
    expect(body.thinking).toEqual({ type: 'disabled' });
  });

  it('uses low reasoning effort for Kimi K3 vision descriptions', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ choices: [{ message: { content: 'black-haired chibi character' } }] }),
    });

    await analyzePhotoWithSettings('data:image/jpeg;base64,abc', {
      ...settingsFor('kimi'),
      visionModel: 'kimi-k3',
    });

    const body = getRequestBody();
    expect(body.model).toBe('kimi-k3');
    expect(body.thinking).toBeUndefined();
    expect(body.reasoning_effort).toBe('low');
    expect(body.max_tokens).toBe(512);
  });

  it('extracts text when a vision response returns content parts', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        choices: [{ message: { content: [{ type: 'text', text: 'blue-eyed chibi character' }] } }],
      }),
    });

    const result = await analyzePhotoWithSettings('data:image/jpeg;base64,abc', {
      ...settingsFor('kimi'),
      visionModel: 'kimi-k2.6',
    });

    expect(result).toBe('blue-eyed chibi character');
  });

  it('rejects an empty vision response instead of returning a blank description', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ choices: [{ message: { content: '' } }] }),
    });

    await expect(
      analyzePhotoWithSettings('data:image/jpeg;base64,abc', settingsFor('deepseek'))
    ).rejects.toThrow('returned an empty description');
  });
});
