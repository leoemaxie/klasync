/**
 * Safe, structured markdown parser for study notes
 */
export function renderMarkdown(md: string): string {
  if (!md?.trim()) return '';

  const lines = md.split('\n');
  const out: string[] = [];
  let inCodeBlock = false;
  let codeBuffer: string[] = [];
  let inList: 'ul' | 'ol' | null = null;

  function closeList() {
    if (inList) {
      out.push(inList === 'ul' ? '</ul>' : '</ol>');
      inList = null;
    }
  }

  function inlineFormat(text: string): string {
    return text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/`([^`]+)`/g, '<code class="md-inline-code">$1</code>')
      .replace(/\*\*(.*?)\*\*/g, '<strong class="md-bold">$1</strong>')
      .replace(/\*(.*?)\*/g, '<em class="md-italic">$1</em>');
  }

  for (const rawLine of lines) {
    const trimmed = rawLine.trim();

    // Code blocks
    if (trimmed.startsWith('```')) {
      closeList();
      if (inCodeBlock) {
        out.push(`<pre class="md-code-block"><code>${codeBuffer.join('\n')}</code></pre>`);
        codeBuffer = [];
        inCodeBlock = false;
      } else {
        inCodeBlock = true;
      }
      continue;
    }
    if (inCodeBlock) {
      codeBuffer.push(rawLine.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;'));
      continue;
    }

    // Skip empty lines (and close list)
    if (!trimmed) {
      closeList();
      continue;
    }

    // Headers
    if (trimmed.startsWith('# ')) {
      closeList();
      out.push(`<h1 class="md-h1">${inlineFormat(trimmed.slice(2))}</h1>`);
    } else if (trimmed.startsWith('## ')) {
      closeList();
      out.push(`<h2 class="md-h2">${inlineFormat(trimmed.slice(3))}</h2>`);
    } else if (trimmed.startsWith('### ')) {
      closeList();
      out.push(`<h3 class="md-h3">${inlineFormat(trimmed.slice(4))}</h3>`);
    } else if (trimmed.startsWith('> ')) {
      closeList();
      out.push(`<blockquote class="md-quote">${inlineFormat(trimmed.slice(2))}</blockquote>`);
    } else if (/^[-*]\s+/.test(trimmed)) {
      // Unordered list (skip if content is empty)
      const content = trimmed.replace(/^[-*]\s+/, '').trim();
      if (content) {
        if (inList !== 'ul') {
          closeList();
          out.push('<ul class="md-ul">');
          inList = 'ul';
        }
        out.push(`<li class="md-li"><span class="md-bullet"></span><span class="md-li-content">${inlineFormat(content)}</span></li>`);
      }
    } else if (/^\d+\.\s+/.test(trimmed)) {
      // Ordered list
      const num = trimmed.match(/^(\d+)\.\s+/)?.[1] || '1';
      const content = trimmed.replace(/^\d+\.\s+/, '').trim();
      if (content) {
        if (inList !== 'ol') {
          closeList();
          out.push('<ol class="md-ol">');
          inList = 'ol';
        }
        out.push(`<li class="md-li-num"><span class="md-num-badge">${num}</span><span class="md-li-content">${inlineFormat(content)}</span></li>`);
      }
    } else {
      closeList();
      out.push(`<p class="md-p">${inlineFormat(trimmed)}</p>`);
    }
  }

  closeList();
  if (inCodeBlock && codeBuffer.length) {
    out.push(`<pre class="md-code-block"><code>${codeBuffer.join('\n')}</code></pre>`);
  }

  return out.join('\n');
}
