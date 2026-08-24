import { beforeEach, describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { PET_STATE_CATALOG, PET_STATES } from '../../../../types/pet';

const { mockAppDataDir, mockJoin, mockConvertFileSrc } = vi.hoisted(() => ({
  mockAppDataDir: vi.fn().mockResolvedValue('C:\\AppData\\Roaming\\desktop-pet'),
  mockJoin: vi.fn((...parts: string[]) => Promise.resolve(parts.join('/'))),
  mockConvertFileSrc: vi.fn((path: string) => `asset://localhost/${path}`),
}));

vi.mock('@tauri-apps/api/path', () => ({
  appDataDir: mockAppDataDir,
  join: mockJoin,
}));
vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: mockConvertFileSrc,
}));

import PreviewStep from '../PreviewStep';

describe('PreviewStep', () => {
  const expectedStates = ['idle', 'sleeping', 'acting_cute', 'working'] as const;

  const defaultProps = {
    petId: 'abc-123',
    onNext: vi.fn(),
    onBack: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders catalog labels in PET_STATES order and loads each state path', async () => {
    const { findByText } = render(<PreviewStep {...defaultProps} />);
    expect(PET_STATES).toEqual(expectedStates);
    for (const state of PET_STATES) {
      const label = PET_STATE_CATALOG.find((definition) => definition.key === state)!.label;
      expect(await findByText(label)).toBeTruthy();
    }
    expect(mockAppDataDir).toHaveBeenCalled();
    const generatedPaths = PET_STATES.map((state) => `${state}.png`);
    expect(generatedPaths).toContain('acting_cute.png');
    expect(mockJoin.mock.calls.map((call) => call[3])).toEqual(generatedPaths);

    const generatedAbsolutePaths = PET_STATES.map(
      (state) => `C:\\AppData\\Roaming\\desktop-pet/pets/abc-123/${state}.png`,
    );
    expect(generatedAbsolutePaths).toContain(
      'C:\\AppData\\Roaming\\desktop-pet/pets/abc-123/acting_cute.png',
    );
    expect(mockConvertFileSrc.mock.calls.map((call) => call[0])).toEqual(generatedAbsolutePaths);
  });

  it('renders four PNG preview images', async () => {
    const { findAllByRole } = render(<PreviewStep {...defaultProps} />);
    const images = await findAllByRole('img');
    expect(images.length).toBe(4);
  });

  it('loads staged state paths when a runId is supplied', async () => {
    render(<PreviewStep {...defaultProps} runId="run-1" />);

    await screen.findAllByRole('img');

    const stagedPaths = PET_STATES.map(
      (state) => `C:\\AppData\\Roaming\\desktop-pet/runs/run-1/selected/${state}.png`,
    );
    expect(stagedPaths).toContain(
      'C:\\AppData\\Roaming\\desktop-pet/runs/run-1/selected/acting_cute.png',
    );
    expect(mockConvertFileSrc.mock.calls.map((call) => call[0])).toEqual(stagedPaths);
  });

  it('renders Next and Back buttons', () => {
    render(<PreviewStep {...defaultProps} />);
    expect(screen.getByRole('button', { name: /下一步/ })).toBeTruthy();
    expect(screen.getByRole('button', { name: /上一步/ })).toBeTruthy();
  });
});
