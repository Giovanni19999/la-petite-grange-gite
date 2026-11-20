// Checks the production build (run after `ng build`):
//   F021-S01: the home page HTML is rendered on the server and contains the site name.
//   F021-S02: the global stylesheet forces the light theme and has no dark variant.
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const browserDir = join(import.meta.dirname, '..', 'dist', 'Front', 'browser');
const failures = [];

const html = readFileSync(join(browserDir, 'index.html'), 'utf8');
if (!/<h1[^>]*>\s*La Petite Grange\s*<\/h1>/.test(html)) {
  failures.push('F021-S01: index.html does not contain the pre-rendered site name.');
}
if (!html.includes('<meta name="color-scheme" content="light"')) {
  failures.push('F021-S02: index.html does not declare color-scheme light.');
}

const stylesFile = readdirSync(browserDir).find((name) => /^styles-.*\.css$/.test(name));
const css = stylesFile ? readFileSync(join(browserDir, stylesFile), 'utf8') : '';
if (!/color-scheme:\s*light/.test(css)) {
  failures.push('F021-S02: the stylesheet does not set color-scheme: light.');
}
if (/\.dark\b|prefers-color-scheme:\s*dark/.test(css)) {
  failures.push('F021-S02: the stylesheet contains a dark theme.');
}

if (failures.length > 0) {
  console.error(failures.join('\n'));
  process.exit(1);
}
console.log('SSR and theme checks passed (F021-S01, F021-S02).');
