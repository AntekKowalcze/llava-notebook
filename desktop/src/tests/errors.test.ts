import { describe, expect, it } from 'vitest';
import { errorKey, errorPayload, isError } from '../lib/errors';

describe('errorKey', () => {
  it('reads unit variants serialised as strings', () => {
    expect(errorKey('SyncFailed')).toBe('SyncFailed');
    expect(isError('SyncFailed', 'SyncFailed')).toBe(true);
  });

  it('reads tuple variants serialised as objects', () => {
    expect(errorKey({ AccountLocked: 5 })).toBe('AccountLocked');
    expect(errorPayload<number>({ AccountLocked: 5 })).toBe(5);
  });

  it('returns an empty key for anything else', () => {
    expect(errorKey(null)).toBe('');
    expect(errorKey(undefined)).toBe('');
    expect(errorKey(new Error('boom'))).toBe('');
    expect(errorKey({ a: 1, b: 2 })).toBe('');
    expect(errorPayload('SyncFailed')).toBeUndefined();
  });
});
