import { sanitizeUrl as reference } from '@braintree/sanitize-url';
import { describe, expect, it } from 'vitest';
import { internalLink, linkUrl, sanitizeUrl } from '../../src/util/url.js';

// Mermaid sanitizes click targets with @braintree/sanitize-url; Pelers must give the same answers,
// apart from keeping percent escapes.
const URLS = [
  'https://example.com',
  'http://EXAMPLE.com/Path?q=1#frag',
  'click.html',
  './relative/path',
  '/absolute/path',
  'mailto:someone@example.com',
  'obsidian://open?vault=Notes&file=Idea',
  'javascript:alert(1)',
  'JaVaScRiPt:alert(1)',
  '  javascript:alert(1)',
  'java\nscript:alert(1)',
  'jav&#x09;ascript:alert(1)',
  '&#106;avascript:alert(1)',
  'data:text/html,<script>alert(1)</script>',
  'vbscript:msgbox(1)',
  'javascript&colon;alert(1)',
  '%6A%61%76%61%73%63%72%69%70%74:alert(1)',
  'https://example.com/a b',
  'http://',
  'ftp://example.com/file',
  'www.example.com',
  'C:\\Users\\file.txt',
  '\\\\server\\share',
  'note name with spaces',
  '#heading',
  '?query=1',
  '',
  'about:blank',
  'tel:+123456',
  'https://example.com/%E0%A4%A',
];

describe('url sanitizer', () => {
  for (const url of URLS) {
    it(`matches the reference for ${JSON.stringify(url)}`, () => {
      expect(sanitizeUrl(url)).toBe(reference(url));
    });
  }
});

describe('percent escapes', () => {
  it('are kept', () => {
    for (const url of [
      'obsidian://open?vault=V&file=A%26B%23C',
      'https://example.com/?q=a%26b',
      'mailto:a@example.com?subject=a%20b',
      'Note%20name',
      './a%23b.html',
    ]) {
      expect(sanitizeUrl(url)).toBe(url);
    }
  });

  it('cannot hide a scheme or a network path', () => {
    expect(sanitizeUrl('%6A%61%76%61%73%63%72%69%70%74:alert(1)')).toBe('about:blank');
    for (const url of ['fi%6ce:///etc/passwd', '%2F%2Fevil.example/share', 'obs%69dian://open']) {
      expect(linkUrl(sanitizeUrl(url), {}), url).toBeUndefined();
    }
  });
});

describe('internal links', () => {
  it('link note names as written', () => {
    for (const name of ['Note', 'Folder\\note', 'Note%23heading', '👩‍💻', 'Note#heading', '  Note  '])
      expect(internalLink(name, {}), name).toBe(true);
  });

  it('refuse names a browser would read as a script or network address', () => {
    for (const name of [
      'javascript:alert(1)',
      'java\ufffescript:alert(1)',
      '\\\\nevil.example',
      '\t/\\nevil.example',
      '/\t/evil.example',
    ]) {
      expect(internalLink(name, { linkSchemes: ['about'] }), JSON.stringify(name)).toBe(false);
    }
  });
});
