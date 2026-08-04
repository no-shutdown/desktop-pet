import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockFetch = vi.fn();
vi.stubGlobal('fetch', mockFetch);

import { analyzePhoto, buildAnalysisMessages } from '../claude-vision';

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
