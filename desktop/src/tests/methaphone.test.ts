import { describe, it, expect } from 'vitest';
import metaphone from '../lib/metaphone';

describe('metaphone', () => {
  it('basic cases', () => {
    expect(metaphone('test')).toBe('tst');
    expect(metaphone('')).toBe('');
    expect(metaphone('a')).toBe('a');
    expect(metaphone('x')).toBe('s');
    expect(metaphone('v')).toBe('f');
  });

  it('two-char rules', () => {
    expect(metaphone('sh')).toBe('x');
    expect(metaphone('th')).toBe('0');
    expect(metaphone('ph')).toBe('f');
  });

  it('real usecases', () => {
    expect(metaphone('delete')).toBe('tlt');
    expect(metaphone('local')).toBe('lkl');
    expect(metaphone('encrypt')).toBe('enkrpt');
    expect(metaphone('logs')).toBe('lks');
    expect(metaphone('password')).toBe('pswrt');
    expect(metaphone('sync')).toBe('snc');
    expect(metaphone('export')).toBe('eksprt');
    expect(metaphone('ai')).toBe('a');
  });
});
