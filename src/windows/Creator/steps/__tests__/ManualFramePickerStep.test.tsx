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
  static deferNextLoad = false;
  static pendingLoads: MockImage[] = [];

  naturalWidth = MockImage.nextSize.width;
  naturalHeight = MockImage.nextSize.height;
  width = MockImage.nextSize.width;
  height = MockImage.nextSize.height;
  onload: (() => void) | null = null;

  set src(_value: string) {
    if (MockImage.deferNextLoad) {
      MockImage.pendingLoads.push(this);
      return;
    }
    queueMicrotask(() => this.onload?.());
  }
}

let activeFileReader: MockFileReader | undefined;

class MockFileReader {
  static nextResult = 'data:image/png;base64,EXTERNAL';

  result = MockFileReader.nextResult;
  onload: ((event: ProgressEvent<FileReader>) => void) | null = null;
  readAsDataURL = vi.fn();

  constructor() {
    activeFileReader = this;
  }
}

const EMPTY_STATES: Record<PetState, SpriteStateInfo> = {
  idle: { cols: 8, rows: 1, frameCount: 8, frameW: 128, frameH: 128, delayMs: 150 },
  sleeping: { cols: 8, rows: 1, frameCount: 8, frameW: 128, frameH: 128, delayMs: 100 },
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
    runId: 'run-1',
    frameW: 128,
    frameH: 128,
    colGap: 7,
    rowGap: 9,
    layout: 'horizontalRows' as const,
    idleFrames: frameCount,
    sleepingFrames: frameCount,
    wavingFrames: frameCount,
    workingFrames: frameCount,
  };
}

function externalFile() {
  return new File(['external'], 'sheet.png', { type: 'image/png' });
}

function setCanvasBounds(canvas: HTMLCanvasElement) {
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
}

beforeEach(() => {
  vi.clearAllMocks();
  MockImage.nextSize = { width: 1024, height: 512 };
  MockImage.deferNextLoad = false;
  MockImage.pendingLoads = [];
  MockFileReader.nextResult = 'data:image/png;base64,EXTERNAL';
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
    const onNext = vi.fn();
    render(
      <ManualFramePickerStep
        {...defaultProps}
        onNext={onNext}
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
        'stage_frame_selections',
        expect.objectContaining({
          runId: 'run-1',
          frameW: 128,
          frameH: 128,
          colGap: 0,
          rowGap: 0,
          idleCells: Array.from({ length: 8 }, (_, col) => ({ col, row: 0 })),
          sleepingCells: Array.from({ length: 8 }, (_, col) => ({ col, row: 1 })),
          wavingCells: Array.from({ length: 8 }, (_, col) => ({ col, row: 2 })),
          workingCells: Array.from({ length: 8 }, (_, col) => ({ col, row: 3 })),
        }),
      );
    });
    expect(mockInvoke).not.toHaveBeenCalledWith('save_frame_selections', expect.anything());
    expect(onNext).toHaveBeenCalledWith('run-1', EMPTY_STATES, 'run-1');
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
          sleepingFrames: 1,
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
    setCanvasBounds(canvas);

    for (const [index, state] of PET_STATES.entries()) {
      fireEvent.click(screen.getByRole('button', { name: actionButtonName(state, 0) }));
      fireEvent.click(canvas, { clientX: 10 + (index % 3) * 140, clientY: 10 + Math.floor(index / 3) * 80 });
    }

    await waitFor(() => expect(screen.getByRole('button', { name: actionButtonName('idle', 1) })).toBeTruthy());
    expect(gridInputs.map((input) => input.value)).toEqual(['128', '64', '4', '8']);

    fireEvent.click(screen.getByRole('button', { name: /确认导入/ }));
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith(
      'stage_frame_selections',
      expect.objectContaining({
        colGap: 4,
        rowGap: 8,
        idleCells: [{ col: 0, row: 0 }],
        sleepingCells: [{ col: 1, row: 0 }],
        wavingCells: [{ col: 2, row: 0 }],
        workingCells: [{ col: 0, row: 1 }],
      }),
    ));
  });

  it('clears old external selections before saving selections from the replacement image', async () => {
    MockImage.nextSize = { width: 512, height: 256 };
    const { container } = render(<ManualFramePickerStep {...defaultProps} />);
    const input = container.querySelector('input[type="file"]') as HTMLInputElement;

    async function upload(result: string) {
      MockFileReader.nextResult = result;
      fireEvent.change(input, { target: { files: [externalFile()] } });
      await act(async () => {
        activeFileReader?.onload?.({ target: activeFileReader } as unknown as ProgressEvent<FileReader>);
      });
      await waitFor(() => expect(screen.getByRole('button', { name: actionButtonName('idle', 0) })).toBeTruthy());
    }

    await upload('data:image/png;base64,IMAGE_A');
    let canvas = container.querySelector('canvas')!;
    setCanvasBounds(canvas);

    for (const [index, state] of PET_STATES.entries()) {
      fireEvent.click(screen.getByRole('button', { name: actionButtonName(state, 0) }));
      fireEvent.click(canvas, {
        clientX: 10 + (index % 3) * 140,
        clientY: 10 + Math.floor(index / 3) * 80,
      });
    }
    await waitFor(() => expect(screen.getByRole('button', { name: actionButtonName('idle', 1) })).toBeTruthy());

    await upload('data:image/png;base64,IMAGE_B');
    canvas = container.querySelector('canvas')!;
    setCanvasBounds(canvas);
    await waitFor(() => {
      const buttons = screen.getAllByRole('button');
      expect(buttons[buttons.length - 1]).toBeDisabled();
    });

    const replacementCells = [
      { clientX: 430, clientY: 10 },
      { clientX: 290, clientY: 10 },
      { clientX: 150, clientY: 10 },
      { clientX: 290, clientY: 90 },
    ];
    for (const [index, state] of PET_STATES.entries()) {
      fireEvent.click(screen.getByRole('button', { name: actionButtonName(state, 0) }));
      fireEvent.click(canvas, replacementCells[index]);
    }

    const saveButtons = screen.getAllByRole('button');
    fireEvent.click(saveButtons[saveButtons.length - 1]);
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith(
      'stage_frame_selections',
      expect.objectContaining({
        dataUrl: 'data:image/png;base64,IMAGE_B',
        idleCells: [{ col: 3, row: 0 }],
        sleepingCells: [{ col: 2, row: 0 }],
        wavingCells: [{ col: 1, row: 0 }],
        workingCells: [{ col: 2, row: 1 }],
      }),
    ));
  });

  it('does not reapply generated selections after replacing the generated image with an external upload', async () => {
    MockImage.nextSize = { width: 1024, height: 512 };
    const { container } = render(
      <ManualFramePickerStep
        {...defaultProps}
        initialDataUrl="data:image/png;base64,GENERATED"
        initialPetId="run-1"
        initialConfig={horizontalConfig()}
      />,
    );

    await waitFor(() => expect(screen.getByRole('button', { name: actionButtonName('idle', 8) })).toBeTruthy());

    MockImage.nextSize = { width: 512, height: 256 };
    MockFileReader.nextResult = 'data:image/png;base64,EXTERNAL_AFTER_GENERATED';
    const input = container.querySelector('input[type="file"]') as HTMLInputElement;
    fireEvent.change(input, { target: { files: [externalFile()] } });
    await act(async () => {
      activeFileReader?.onload?.({ target: activeFileReader } as unknown as ProgressEvent<FileReader>);
    });

    await waitFor(() => {
      for (const state of PET_STATES) {
        expect(screen.getByRole('button', { name: actionButtonName(state, 0) })).toBeTruthy();
      }
    });
  });

  it('clears canonical selections when the image and initial config are replaced by external data', async () => {
    const { rerender } = render(
      <ManualFramePickerStep
        {...defaultProps}
        initialDataUrl="data:image/png;base64,CANONICAL_A"
        initialPetId="run-1"
        initialConfig={horizontalConfig()}
      />,
    );

    await waitFor(() => expect(screen.getByRole('button', { name: actionButtonName('idle', 8) })).toBeTruthy());

    rerender(
      <ManualFramePickerStep
        {...defaultProps}
        initialDataUrl="data:image/png;base64,EXTERNAL_B"
        initialPetId={null}
        initialConfig={undefined}
      />,
    );

    await waitFor(() => {
      for (const state of PET_STATES) {
        expect(screen.getByRole('button', { name: actionButtonName(state, 0) })).toBeTruthy();
      }
    });
  });

  it('waits for the replacement image before applying a changed canonical config', async () => {
    const { rerender } = render(
      <ManualFramePickerStep
        {...defaultProps}
        initialDataUrl="data:image/png;base64,CANONICAL_A"
        initialPetId="run-1"
        initialConfig={horizontalConfig(8)}
      />,
    );

    await waitFor(() => expect(screen.getByRole('button', { name: actionButtonName('idle', 8) })).toBeTruthy());

    MockImage.deferNextLoad = true;
    rerender(
      <ManualFramePickerStep
        {...defaultProps}
        initialDataUrl="data:image/png;base64,CANONICAL_B"
        initialPetId="run-1"
        initialConfig={horizontalConfig(2)}
      />,
    );

    await waitFor(() => {
      const buttons = screen.getAllByRole('button');
      expect(buttons[buttons.length - 1]).toBeDisabled();
    });

    MockImage.deferNextLoad = false;
    await act(async () => {
      for (const image of MockImage.pendingLoads) {
        image.onload?.();
      }
    });

    await waitFor(() => {
      for (const state of PET_STATES) {
        expect(screen.getByRole('button', { name: actionButtonName(state, 2) })).toBeTruthy();
      }
    });
  });
});
