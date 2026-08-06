import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { PET_STATE_CATALOG, PET_STATES, type PetState, type SpriteStateInfo } from '../../../../types/pet';

const { mockInvoke } = vi.hoisted(() => ({ mockInvoke: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({ invoke: mockInvoke }));

import ManualFramePickerStep from '../ManualFramePickerStep';

const mockContext = {
  beginPath: vi.fn(),
  clearRect: vi.fn(),
  drawImage: vi.fn(),
  fillRect: vi.fn(),
  fillText: vi.fn(),
  lineTo: vi.fn(),
  moveTo: vi.fn(),
  stroke: vi.fn(),
  strokeRect: vi.fn(),
};

const originalGetContext = HTMLCanvasElement.prototype.getContext;

class MockImage {
  static nextSize = { width: 1024, height: 512 };

  naturalWidth = MockImage.nextSize.width;
  naturalHeight = MockImage.nextSize.height;
  width = MockImage.nextSize.width;
  height = MockImage.nextSize.height;
  onload: (() => void) | null = null;

  set src(_value: string) {
    queueMicrotask(() => this.onload?.());
  }
}

let activeFileReader: MockFileReader | undefined;

class MockFileReader {
  result = 'data:image/png;base64,EXTERNAL';
  onload: ((event: ProgressEvent<FileReader>) => void) | null = null;
  readAsDataURL = vi.fn();

  constructor() {
    activeFileReader = this;
  }
}

const EMPTY_STATES: Record<PetState, SpriteStateInfo> = {
  idle: { cols: 8, rows: 1, frameCount: 8, frameW: 128, frameH: 128, delayMs: 150 },
  walking: { cols: 8, rows: 1, frameCount: 8, frameW: 128, frameH: 128, delayMs: 100 },
  waving: { cols: 8, rows: 1, frameCount: 8, frameW: 128, frameH: 128, delayMs: 110 },
  working: { cols: 8, rows: 1, frameCount: 8, frameW: 128, frameH: 128, delayMs: 120 },
};

const defaultProps = {
  onNext: vi.fn(),
  onBack: vi.fn(),
};

function actionButtonName(state: PetState, count: number) {
  const label = PET_STATE_CATALOG.find((definition) => definition.key === state)!.label;
  return new RegExp(`${label}\\(${count}\\)`);
}

function horizontalConfig(frameCount = 8) {
  return {
    frameW: 128,
    frameH: 128,
    colGap: 7,
    rowGap: 9,
    layout: 'horizontalRows' as const,
    idleFrames: frameCount,
    walkingFrames: frameCount,
    wavingFrames: frameCount,
    workingFrames: frameCount,
  };
}

function externalFile() {
  return new File(['external'], 'sheet.png', { type: 'image/png' });
}

beforeEach(() => {
  vi.clearAllMocks();
  MockImage.nextSize = { width: 1024, height: 512 };
  mockInvoke.mockResolvedValue(EMPTY_STATES);
  activeFileReader = undefined;
  vi.stubGlobal('Image', MockImage);
  vi.stubGlobal('FileReader', MockFileReader);
  Object.defineProperty(HTMLCanvasElement.prototype, 'getContext', {
    configurable: true,
    value: vi.fn().mockReturnValue(mockContext),
  });
});

afterEach(() => {
  vi.unstubAllGlobals();
  Object.defineProperty(HTMLCanvasElement.prototype, 'getContext', {
    configurable: true,
    value: originalGetContext,
  });
});

describe('ManualFramePickerStep', () => {
  it('preselects eight cells per canonical row and saves zero gaps', async () => {
    render(
      <ManualFramePickerStep
        {...defaultProps}
        initialDataUrl="data:image/png;base64,COMBINED"
        initialPetId="run-1"
        initialConfig={horizontalConfig()}
      />,
    );

    await waitFor(() => {
      for (const state of PET_STATES) {
        expect(screen.getByRole('button', { name: actionButtonName(state, 8) })).toBeTruthy();
      }
    });

    fireEvent.click(screen.getByRole('button', { name: /确认导入/ }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        'save_frame_selections',
        expect.objectContaining({
          petId: 'run-1',
          frameW: 128,
          frameH: 128,
          colGap: 0,
          rowGap: 0,
          idleCells: Array.from({ length: 8 }, (_, col) => ({ col, row: 0 })),
          walkingCells: Array.from({ length: 8 }, (_, col) => ({ col, row: 1 })),
          wavingCells: Array.from({ length: 8 }, (_, col) => ({ col, row: 2 })),
          workingCells: Array.from({ length: 8 }, (_, col) => ({ col, row: 3 })),
        }),
      );
    });
  });

  it('clears stale selections and applies a changed layout config', async () => {
    const { rerender } = render(
      <ManualFramePickerStep
        {...defaultProps}
        initialDataUrl="data:image/png;base64,COMBINED"
        initialPetId="run-1"
        initialConfig={horizontalConfig()}
      />,
    );

    await waitFor(() => expect(screen.getByRole('button', { name: actionButtonName('idle', 8) })).toBeTruthy());

    rerender(
      <ManualFramePickerStep
        {...defaultProps}
        initialDataUrl="data:image/png;base64,COMBINED"
        initialPetId="run-1"
        initialConfig={{
          frameW: 96,
          frameH: 80,
          colGap: 4,
          rowGap: 6,
          layout: 'grid',
          idleFrames: 1,
          walkingFrames: 1,
          wavingFrames: 1,
          workingFrames: 1,
        }}
      />,
    );

    await waitFor(() => {
      for (const state of PET_STATES) {
        expect(screen.getByRole('button', { name: actionButtonName(state, 1) })).toBeTruthy();
      }
      expect(screen.queryByRole('button', { name: actionButtonName('idle', 8) })).toBeNull();
    });

    const gridInputs = screen.getAllByRole('spinbutton') as HTMLInputElement[];
    expect(gridInputs.map((input) => input.value)).toEqual(['96', '80', '4', '6']);
  });

  it('preserves external grid gaps and allows selecting cells after upload', async () => {
    MockImage.nextSize = { width: 512, height: 256 };
    const { container } = render(<ManualFramePickerStep {...defaultProps} />);
    const input = container.querySelector('input[type="file"]') as HTMLInputElement;

    fireEvent.change(input, { target: { files: [externalFile()] } });
    await act(async () => {
      activeFileReader?.onload?.({ target: activeFileReader } as unknown as ProgressEvent<FileReader>);
    });

    await waitFor(() => expect(screen.getByRole('button', { name: actionButtonName('idle', 0) })).toBeTruthy());

    const gridInputs = screen.getAllByRole('spinbutton') as HTMLInputElement[];
    fireEvent.change(gridInputs[2], { target: { value: '4' } });
    fireEvent.change(gridInputs[3], { target: { value: '8' } });

    const canvas = container.querySelector('canvas')!;
    Object.defineProperty(canvas, 'getBoundingClientRect', {
      configurable: true,
      value: () => ({
        bottom: canvas.height,
        height: canvas.height,
        left: 0,
        right: canvas.width,
        top: 0,
        width: canvas.width,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      }),
    });

    for (const [index, state] of PET_STATES.entries()) {
      fireEvent.click(screen.getByRole('button', { name: actionButtonName(state, 0) }));
      fireEvent.click(canvas, { clientX: 10 + (index % 3) * 140, clientY: 10 + Math.floor(index / 3) * 80 });
    }

    await waitFor(() => expect(screen.getByRole('button', { name: actionButtonName('idle', 1) })).toBeTruthy());
    expect(gridInputs.map((input) => input.value)).toEqual(['128', '64', '4', '8']);

    fireEvent.click(screen.getByRole('button', { name: /确认导入/ }));
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith(
      'save_frame_selections',
      expect.objectContaining({
        colGap: 4,
        rowGap: 8,
        idleCells: [{ col: 0, row: 0 }],
        walkingCells: [{ col: 1, row: 0 }],
        wavingCells: [{ col: 2, row: 0 }],
        workingCells: [{ col: 0, row: 1 }],
      }),
    ));
  });
});
