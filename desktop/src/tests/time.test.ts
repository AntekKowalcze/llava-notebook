import { describe, expect, it } from 'vitest';
import { formatTimeAgo } from '../lib/time';

const DAY = 86_400_000;

describe('formatTimeAgo', () => {
  it('formats each range', () => {
    expect(formatTimeAgo(1000, 1000 + 30_000)).toBe('just now');
    expect(formatTimeAgo(0, 60_000)).toBe('1 minute ago');
    expect(formatTimeAgo(0, 3 * 3_600_000)).toBe('3 hours ago');
    expect(formatTimeAgo(0, 2 * DAY)).toBe('2 days ago');
    expect(formatTimeAgo(0, 14 * DAY)).toBe('2 weeks ago');
    expect(formatTimeAgo(0, 90 * DAY)).toBe('3 months ago');
    expect(formatTimeAgo(0, 366 * DAY)).toBe('1 year ago');
    expect(formatTimeAgo(0, 800 * DAY)).toBe('2 years ago');
  });

  it('never goes negative for timestamps in the future', () => {
    expect(formatTimeAgo(10_000, 0)).toBe('just now');
  });
});
