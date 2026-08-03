import { describe, it, expect } from 'vitest';

function resolveComponent(label: string): 'Pet' | 'Creator' {
  return label === 'pet' ? 'Pet' : 'Creator';
}

describe('window routing', () => {
  it('routes "pet" label to Pet', () => {
    expect(resolveComponent('pet')).toBe('Pet');
  });
  it('routes "creator" label to Creator', () => {
    expect(resolveComponent('creator')).toBe('Creator');
  });
  it('routes unknown labels to Creator', () => {
    expect(resolveComponent('anything')).toBe('Creator');
  });
});
