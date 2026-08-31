import { describe, expect, it } from 'vitest';
import { INITIAL_WIZARD_DATA } from '../types';

describe('WizardData', () => {
  it('starts without a base-image style reference', () => {
    expect(INITIAL_WIZARD_DATA.styleReferenceDataUrl).toBeNull();
  });
});
