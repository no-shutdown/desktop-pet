import { beforeEach, describe, expect, it } from 'vitest';
import { fireEvent, render } from '@testing-library/react';
import { createElement } from 'react';
import SettingsPanel from '../../windows/Creator/SettingsPanel';
import {
  DEFAULT_SETTINGS,
  IMAGE_PROVIDERS,
  SILICONFLOW_BASE_MODELS,
  SILICONFLOW_REFERENCE_MODELS,
  loadSettings,
  normalizeDenoisingStrength,
  saveSettings,
} from '../settings';

const STORAGE_KEY = 'desktop-pet-settings';

describe('settings defaults and migration', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('defaults image generation to SiliconFlow with separate base/reference models', () => {
    const settings = loadSettings();

    expect(settings.imageProvider).toBe('siliconflow');
    expect(settings.imageBaseModel).toBe('Tongyi-MAI/Z-Image-Turbo');
    expect(settings.imageReferenceModel).toBe('Qwen/Qwen-Image-Edit-2509');
    expect(settings.imageModel).toBe(settings.imageBaseModel);
    expect(settings.localSdDenoisingStrength).toBe(0.55);
    expect(DEFAULT_SETTINGS.imageProvider).toBe('siliconflow');
  });

  it('round-trips the new provider and model settings while keeping imageModel compatible', () => {
    const settings = loadSettings();
    const updated = {
      ...settings,
      imageProvider: 'localsd' as const,
      imageBaseModel: 'Tongyi-MAI/Z-Image',
      imageReferenceModel: 'Kwai-Kolors/Kolors',
      localSdUrl: 'http://localhost:7861',
      localSdDenoisingStrength: 0.65,
    };

    saveSettings(updated);

    expect(JSON.parse(localStorage.getItem(STORAGE_KEY)!)).toMatchObject({
      imageProvider: 'localsd',
      imageBaseModel: 'Tongyi-MAI/Z-Image',
      imageReferenceModel: 'Kwai-Kolors/Kolors',
      imageModel: 'Tongyi-MAI/Z-Image',
      localSdDenoisingStrength: 0.65,
    });
    expect(loadSettings()).toMatchObject({
      ...updated,
      imageModel: updated.imageBaseModel,
    });
    expect(loadSettings().imageModel).toBe('Tongyi-MAI/Z-Image');
  });

  it('migrates legacy Pollinations settings to SiliconFlow without losing the API key', () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({
      imageProvider: 'pollinations',
      imageApiKey: 'sf-key',
      imageModel: 'Tongyi-MAI/Z-Image-Turbo',
    }));

    const settings = loadSettings();

    expect(settings.imageProvider).toBe('siliconflow');
    expect(settings.imageApiKey).toBe('sf-key');
    expect(settings.imageBaseModel).toBe('Tongyi-MAI/Z-Image-Turbo');
    expect(settings.imageModel).toBe('Tongyi-MAI/Z-Image-Turbo');
  });

  it('copies a legacy imageModel into the new base model field', () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({
      imageProvider: 'siliconflow',
      imageModel: 'legacy/base-model',
    }));

    expect(loadSettings().imageBaseModel).toBe('legacy/base-model');
  });

  it('keeps a selected Base model after saving and reloading', () => {
    const settings = loadSettings();
    const selectedBaseModel = 'Tongyi-MAI/Z-Image';

    saveSettings({
      ...settings,
      imageModel: selectedBaseModel,
      imageBaseModel: selectedBaseModel,
    });

    const reloaded = loadSettings();
    expect(reloaded.imageBaseModel).toBe(selectedBaseModel);
    expect(reloaded.imageModel).toBe(selectedBaseModel);
  });

  it('normalizes absent, partial, invalid, and out-of-range stored values', () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({
      imageProvider: 'not-a-provider',
      imageApiKey: null,
      imageBaseModel: 42,
      imageReferenceModel: '',
      localSdUrl: null,
      localSdDenoisingStrength: 99,
    }));

    expect(loadSettings()).toMatchObject({
      imageProvider: 'siliconflow',
      imageApiKey: '',
      imageBaseModel: 'Tongyi-MAI/Z-Image-Turbo',
      imageReferenceModel: 'Qwen/Qwen-Image-Edit-2509',
      localSdUrl: 'http://localhost:7860',
      localSdDenoisingStrength: 0.75,
    });

    localStorage.setItem(STORAGE_KEY, '{not valid json');
    expect(loadSettings()).toEqual(DEFAULT_SETTINGS);
  });
});

describe('image generation settings contract', () => {
  it('exposes only SiliconFlow and Local SD providers', () => {
    expect(IMAGE_PROVIDERS).toEqual(['siliconflow', 'localsd']);
    expect(SILICONFLOW_BASE_MODELS.map(({ value }) => value)).toContain('Tongyi-MAI/Z-Image-Turbo');
    expect(SILICONFLOW_REFERENCE_MODELS.map(({ value }) => value)).toContain('Qwen/Qwen-Image-Edit-2509');
  });

  it('clamps denoising strength to the Local SD safety range', () => {
    expect(normalizeDenoisingStrength(-1)).toBe(0.35);
    expect(normalizeDenoisingStrength(0.55)).toBe(0.55);
    expect(normalizeDenoisingStrength(1)).toBe(0.75);
    expect(normalizeDenoisingStrength(Number.NaN)).toBe(0.55);
  });

  it('persists a normalized denoising strength', () => {
    saveSettings({ ...loadSettings(), localSdDenoisingStrength: 0.1 });

    expect(JSON.parse(localStorage.getItem(STORAGE_KEY)!)).toHaveProperty('localSdDenoisingStrength', 0.35);
    expect(loadSettings().localSdDenoisingStrength).toBe(0.35);
  });
});

describe('SettingsPanel image generation controls', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('renders two provider choices, separate models, and Local SD denoising controls', () => {
    const { container } = render(createElement(SettingsPanel, { onBack: () => {} }));

    expect(container.querySelectorAll('input[name="imageProvider"]')).toHaveLength(2);
    expect(container.querySelector('input[value="pollinations"]')).toBeNull();
    expect(container.querySelector('select[aria-label="Base model"]')).not.toBeNull();
    expect(container.querySelector('select[aria-label="Reference model"]')).not.toBeNull();

    fireEvent.click(container.querySelector('input[value="localsd"]')!);

    const slider = container.querySelector('input[name="localSdDenoisingStrength"]');
    expect(slider).toHaveAttribute('type', 'range');
    expect(slider).toHaveAttribute('min', '0.35');
    expect(slider).toHaveAttribute('max', '0.75');
    expect(slider).toHaveAttribute('step', '0.05');
  });
});
