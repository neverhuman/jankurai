// Public audit-demo catalog. README embeds the preview; the 1080p file is
// the full-resolution sibling. Both are exact-palette GIF89a, under 50 MB.
export const MAX_BYTES = 50 * 1024 * 1024;
export const PUBLIC_DIR = 'docs/demo';
export const RECEIPT = 'audit-demo.json';
export const PRESETS = Object.freeze([
  Object.freeze({ name: 'audit-readme.gif', width: 960, height: 540, role: 'readme-preview' }),
  Object.freeze({ name: 'audit-1080p.gif', width: 1920, height: 1080, role: 'full-resolution' }),
]);
export const PUBLIC_FILES = Object.freeze([...PRESETS.map((preset) => preset.name), RECEIPT]);
