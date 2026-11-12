import { assertWellFormed, elements } from './xml.js';

// Pelers's output is inserted into pages as markup, so it must be inert whatever the diagram says:
// only drawing elements, only presentation attributes, no script-capable URLs, no way to load a stylesheet.
// A new diagram type that needs another element or attribute adds it here, with care.

const ELEMENTS = new Set([
  'svg',
  'g',
  'rect',
  'circle',
  'ellipse',
  'polygon',
  'path',
  'text',
  'tspan',
  'a',
  'title',
  'desc',
  'image',
]);
const ATTRIBUTES = new Set([
  'xmlns',
  'class',
  'viewBox',
  'width',
  'height',
  'style',
  'font-family',
  'font-size',
  'font-weight',
  'font-style',
  'fill',
  'fill-opacity',
  'opacity',
  'stroke',
  'stroke-width',
  'stroke-dasharray',
  'stroke-linecap',
  'stroke-linejoin',
  'role',
  'aria-roledescription',
  'aria-labelledby',
  'aria-describedby',
  'id',
  'data-id',
  'data-icon',
  'data-href',
  'transform',
  'x',
  'y',
  'rx',
  'ry',
  'r',
  'cx',
  'cy',
  'd',
  'points',
  'text-anchor',
  'xml:space',
  'href',
  'target',
  'rel',
  'preserveAspectRatio',
  'pointer-events',
]);
const STYLE_PROPERTIES = new Set([
  'fill',
  'fill-opacity',
  'stroke',
  'stroke-width',
  'stroke-dasharray',
  'stroke-dashoffset',
  'stroke-linecap',
  'stroke-linejoin',
  'stroke-opacity',
  'opacity',
  'rx',
  'ry',
  'font-size',
  'font-family',
  'font-weight',
  'font-style',
  'text-decoration',
  'letter-spacing',
  'word-spacing',
  'max-width',
]);
const ROOT_STYLES = new Set(['height:auto', 'color:var(--_fg)', 'text-transform:none', 'direction:ltr']);
const RE_ROOT_SERIES = /^--pelers-series-[1-8]$/;
const RE_ROOT_FONT = /^font:\d+(?:\.\d+)?(?:e[-+]?\d+)?px sans-serif$/;
const NUMERIC = new Set(['x', 'y', 'width', 'height', 'r', 'cx', 'cy', 'rx', 'ry']);

const RE_NUMBER = /^(?:-?\d+(?:\.\d+)?(?:e[-+]?\d+)?|100%)$/;
const RE_SCRIPT_URL = /^[\s\u0000-\u001f]*(?:javascript|vbscript|data)\s*:/i;
const RE_ALLOWED_URL = /^(?:(?:https?|mailto|tel):|(?![\\/]{2}|[a-z][a-z0-9+.-]*:))/i;
const RE_TARGET = /^_(?:self|blank|parent|top)$/;
const RE_UNSAFE_STYLE = /url\s*\(|expression|@import|javascript:|[<>{}\\]/i;
const RE_UNSAFE_PAINT = /url\s*\(|javascript:/i;

// The first thing about the markup that is not inert, or undefined when there is none.
export function inertProblem(svg: string): string | undefined {
  try {
    assertWellFormed(svg);
  } catch (error) {
    return (error as Error).message;
  }
  for (const el of elements(svg)) {
    if (!ELEMENTS.has(el.name)) return `element <${el.name}>`;
    for (const [name, value] of el.attrs) {
      if (!ATTRIBUTES.has(name)) return `attribute ${name} on <${el.name}>`;
      if (NUMERIC.has(name) && !RE_NUMBER.test(value)) return `${name}="${value}" is not a number`;
      if (name === 'href') {
        if (el.name !== 'a' && el.name !== 'image') return `href on <${el.name}>`;
        if (RE_SCRIPT_URL.test(value)) return `href="${value}" can run script`;
        // What a browser makes of the value: tabs and line breaks dropped, then leading spaces and controls.
        const seen = value.replace(/[\t\n\r]/g, '').replace(/^[\u0000-\u0020]+/, '');
        if (!RE_ALLOWED_URL.test(seen)) return `href="${value}" has a scheme that is not allowed`;
      } else if (name === 'target') {
        if (!RE_TARGET.test(value)) return `target="${value}"`;
      } else if (name === 'style') {
        if (RE_UNSAFE_STYLE.test(value)) return `style="${value}"`;
        for (const decl of value.split(';')) {
          const trimmed = decl.trim();
          if (trimmed === '') continue;
          const prop = decl.slice(0, decl.indexOf(':')).trim();
          const root =
            el.name === 'svg' && (ROOT_STYLES.has(trimmed) || RE_ROOT_FONT.test(trimmed) || RE_ROOT_SERIES.test(prop));
          if (!root && !STYLE_PROPERTIES.has(prop) && !prop.startsWith('--_')) return `style property ${prop}`;
        }
      } else if (name === 'fill' || name === 'stroke' || name === 'font-family' || name === 'rx') {
        if (RE_UNSAFE_PAINT.test(value)) return `${name}="${value}"`;
      }
    }
  }
  return undefined;
}

// Checked with plain conditions: an assertion call for every attribute made these tests many
// times slower than the rendering they check.
export function assertInert(svg: string, where: string): void {
  const problem = inertProblem(svg);
  if (problem !== undefined) throw new Error(`${where}: ${problem}`);
}

// Text an attacker might put anywhere a diagram accepts a string.
export const PAYLOADS = [
  '<script>alert(1)</script>',
  '"><script>alert(1)</script>',
  '<img src=x onerror=alert(1)>',
  '<svg onload=alert(1)>',
  '<a href="javascript:alert(1)">x</a>',
  '<foreignObject><iframe src=javascript:alert(1)></iframe></foreignObject>',
  '</text></g></svg><script>alert(1)</script>',
  'javascript:alert(1)',
  'JaVaScRiPt:alert(1)',
  ' javascript:alert(1)',
  'jav&#x09;ascript:alert(1)',
  'data:text/html,<script>alert(1)</script>',
  'vbscript:msgbox(1)',
  "' onload='alert(1)",
  '" onmouseover="alert(1)',
  'x" onclick="alert(1)" y="',
  '#60;script#62;alert(1)#60;/script#62;',
  '#106;avascript:alert(1)',
  '#34; onload=#34;alert(1)',
  'url#40;javascript:alert#40;1#41;#41;',
  'red#59;background:url#40;//evil.example/x#41;',
  '&lt;script&gt;alert(1)&lt;/script&gt;',
  '&#60;script&#62;',
  'url(javascript:alert(1))',
  'expression(alert(1))',
  'red;background:url(//evil.example/x)',
  'red" onload="alert(1)',
  ']]><script>alert(1)</script>',
  '--><script>alert(1)</script>',
  '\u0000\u0001\u0008\u000b\u001f',
  'a\ufffeb\uffffc\ud800d\udc00e#65534;f',
  'u\u0001rl(//evil.example/x)',
  'u\ufffeRL(//evil.example/x)',
  'u\ud800rl(//evil.example/x)',
  'java\ufffescript:alert(1)',
  'java\udc00script:alert(1)',
  'file\uffff:///etc/passwd',
  '__proto__',
  'constructor',
  'toString',
  'hasOwnProperty',
  '{{7*7}}',
  '${alert(1)}',
  '%0ajavascript:alert(1)',
  'file:///etc/passwd',
  '//evil.example/share',
  '\\\\evil.example\\share',
  '#92;#92;nevil.example/share',
  '\t/#92;nevil.example/share',
  'smb://evil.example/share',
  'obsidian://open?vault=x',
  '\\"><script>',
];
