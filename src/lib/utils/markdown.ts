/**
 * Lightweight safe markdown parser for lecture notes
 */
export function renderMarkdown(md: string): string {
  if (!md) return '';

  // Escape raw HTML entities for safety
  let safe = md
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');

  // Code blocks (```lang ... ```)
  safe = safe.replace(
    /```([\s\S]*?)```/g,
    '<pre class="md-code-block"><code>$1</code></pre>'
  );

  // Inline code (`code`)
  safe = safe.replace(/`([^`]+)`/g, '<code class="md-inline-code">$1</code>');

  // Headers (# H1, ## H2, ### H3)
  safe = safe.replace(/^### (.*$)/gim, '<h3 class="md-h3">$1</h3>');
  safe = safe.replace(/^## (.*$)/gim, '<h2 class="md-h2">$1</h2>');
  safe = safe.replace(/^# (.*$)/gim, '<h1 class="md-h1">$1</h1>');

  // Blockquotes (> quote)
  safe = safe.replace(
    /^\> (.*$)/gim,
    '<blockquote class="md-quote">$1</blockquote>'
  );

  // Bold & Italic (**bold**, *italic*)
  safe = safe.replace(/\*\*(.*?)\*\*/g, '<strong class="md-bold">$1</strong>');
  safe = safe.replace(/\*(.*?)\*/g, '<em class="md-italic">$1</em>');

  // Unordered list items (- item or * item)
  safe = safe.replace(/^\s*[-*]\s+(.*$)/gim, '<li class="md-li">$1</li>');

  // Ordered list items (1. item)
  safe = safe.replace(/^\s*\d+\.\s+(.*$)/gim, '<li class="md-li-num">$1</li>');

  // Paragraph line breaks (double newline = paragraph, single = br)
  safe = safe.replace(/\n\n+/g, '</p><p class="md-p">');
  safe = safe.replace(/\n/g, '<br/>');

  return `<div class="md-root"><p class="md-p">${safe}</p></div>`;
}
