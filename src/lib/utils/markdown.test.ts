import { describe, expect, it } from 'vitest';
import { renderMarkdown } from './markdown';

describe('renderMarkdown timestamps', () => {
  it('turns [mm:ss] into a seek button', () => {
    const html = renderMarkdown('- [01:18] ATP stores energy');
    expect(html).toContain('class="md-time"');
    expect(html).toContain('data-seek="01:18"');
    expect(html).toContain('ATP stores energy');
  });
  it('supports h:mm:ss', () => {
    expect(renderMarkdown('[1:02:03] later')).toContain('data-seek="1:02:03"');
  });
  it('leaves other brackets alone', () => {
    const html = renderMarkdown('see [note] and [1:2] and [abc:de]');
    expect(html).not.toContain('md-time');
  });
  it('still escapes HTML', () => {
    const html = renderMarkdown('<img src=x onerror=alert(1)> [00:05]');
    expect(html).not.toContain('<img');
    expect(html).toContain('&lt;img');
  });
});
