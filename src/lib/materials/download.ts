/** Hand a Blob to the browser as a file download. */
export function saveBlob(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}

const SAFE = /[^a-zA-Z0-9._-]+/g;

/** `Intro to Sets` + `pdf` -> `Intro_to_Sets.pdf`. */
export function exportFilename(title: string, ext: string): string {
  const stem = title
    .trim()
    .replace(SAFE, '_')
    .replace(/^_+|_+$/g, '');
  return `${stem || 'material'}.${ext}`;
}
