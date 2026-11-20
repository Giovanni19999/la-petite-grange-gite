// Checks the production build (run after `ng build`), F021-S02: light theme only.
// Server-side rendering of the pages themselves (F021-S01, F002-S01) needs the API and is
// checked on the running stack by deploy/smoke-test.sh.
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const browserDir = join(import.meta.dirname, '..', 'dist', 'Front', 'browser');
const failures = [];

const html = readFileSync(join(browserDir, 'index.csr.html'), 'utf8');
if (!html.includes('<meta name="color-scheme" content="light"')) {
  failures.push('F021-S02: index.html does not declare color-scheme light.');
}
if (/class="[^"]*\bdark\b/.test(html)) {
  failures.push('F021-S02: the page enables the dark class.');
}

const stylesFile = readdirSync(browserDir).find((name) => /^styles-.*\.css$/.test(name));
const css = stylesFile ? readFileSync(join(browserDir, stylesFile), 'utf8') : '';
if (!/color-scheme:\s*light/.test(css)) {
  failures.push('F021-S02: the stylesheet does not set color-scheme: light.');
}
// spartan/ui components carry inert `dark:` utilities (`:is(.dark *)`), active only under a
// `.dark` class that the site never sets. A dark theme would show up as a media query on the
// device color scheme or as theme variables redefined for `.dark`.
if (/prefers-color-scheme:\s*dark/.test(css)) {
  failures.push('F021-S02: the stylesheet follows the dark color scheme of the device.');
}
if (/(^|[}\s,]):root\.dark|(^|[}\s,])\.dark\s*\{/.test(css)) {
  failures.push('F021-S02: the stylesheet defines dark theme variables.');
}

if (failures.length > 0) {
  console.error(failures.join('\n'));
  process.exit(1);
}
console.log('Theme checks passed (F021-S02).');
