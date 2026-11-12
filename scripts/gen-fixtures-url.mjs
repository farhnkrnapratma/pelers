import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const SCALE = Number(process.env.PELERS_FUZZ_SCALE ?? 1);
const OUT = process.env.PELERS_FUZZ_OUT ?? (join(root, 'tests/fixtures'));
const url = await import(join(root, 'ts/src/util/url.ts'));

const inputs = [
  '', 'https://Example.COM/Path?q=1#frag', 'http://EXAMPLE.org:8080/a b', 'HTTPS://x.y', 'mailto:someone@example.com',
  'tel:+123', 'javascript:alert(1)', 'JaVaScRiPt:alert(1)', ' javascript:alert(1)', 'data:text/html,hi', 'vbscript:msgbox',
  '&#106;avascript:alert(1)', 'java&#x73;cript:x', 'java&Tab;script:x', 'java&newline;script:x', 'java\tscript:x',
  '%6Aavascript:alert(1)', '%6A%61vascript:alert(1)', '/relative/path', './rel', '../up', '\\\\server\\share',
  '\\windows\\path', 'file:///etc/passwd', 'custom-scheme://x/y', 'ftp://host/file', 'https://ex ample.com',
  'https://[::1]:8080/', 'https://xn--nxasmq6b.com/', 'http://example.com/%E4%BD%A0', 'note.md?file=A%26B',
  'a%5cb', 'a\\b\\c', 'x&colon;y', 'https&colon;//a.b', '&#0;javascript:x', '&#65;&#66;', '&#xZZ;',
  'plain text', 'with spaces only   ', '%E4%', '%zz', 'https://example.com/%ZZ', 'data&#58;x',
  '&#1e3;javascript:x', 'mailto:' + 'a'.repeat(50), 'https://a.b/?x=é中', 'abc:def', '1:2',
  'a:b\nc', 'x\r\ny:z', 'https://u:p@h.com/', 'HTTP://UPPER.HOST.COM', '  https://trim.me  ',
];
const out = inputs.map(input => ({ input, sanitize: url.sanitizeUrl(input), safe: url.safeUrl(input) }));
writeFileSync(join(OUT, 'url.json'), `${JSON.stringify(out)}\n`);
console.log(`url: ${out.length} cases`);
