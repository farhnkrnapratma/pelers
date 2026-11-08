import { RE_INVALID_XML, esc } from '../svg/builder.js';
import { decodeEntities } from '../text/entities.js';
import type { LinkInfo, RenderOptions } from '../types.js';

const BLANK = 'about:blank';
const RE_INVALID_PROTOCOL = /^([^\w]*)(javascript|data|vbscript)/im;
const RE_HTML_ENTITIES = /&#(\w+)(^\w|;)?/g;
const RE_HTML_CTRL_ENTITY = /&(newline|tab);/gi;
const RE_CTRL = /[\u0000-\u001F\u007F-\u009F -‍﻿\uD800-\uDFFF\uFFFE\uFFFF]/gimu;
const RE_SCHEME = /^.+(:|&colon;)/gim;
const RE_WS_ESCAPE = /(\\|%5[cC])((%(6[eE]|72|74))|[nrt])/g;

function decode(uri: string): string {
  try {
    return decodeURIComponent(uri);
  } catch {
    return uri;
  }
}

// Removes the control characters, entity codes and escapes that could hide a scheme, and with
// `percent`, percent escapes too. Undefined for a URL wrapped too many times to be a real one.
function unwrap(url: string, percent: boolean): string | undefined {
  let out = percent ? decode(url.trim()) : url.trim();
  let more: RegExpMatchArray | null;
  let rounds = 0;
  do {
    if (++rounds > 32) return undefined;
    out = out
      .replace(RE_CTRL, '')
      .replace(RE_HTML_ENTITIES, (_m, dec) => String.fromCharCode(dec))
      .replace(RE_HTML_CTRL_ENTITY, '')
      .replace(RE_CTRL, '')
      .replace(RE_WS_ESCAPE, '')
      .trim();
    if (percent) out = decode(out);
    more =
      out.match(RE_CTRL) || out.match(RE_HTML_ENTITIES) || out.match(RE_HTML_CTRL_ENTITY) || out.match(RE_WS_ESCAPE);
  } while (more && more.length > 0);
  return out;
}

// Refuses what @braintree/sanitize-url refuses, the check Mermaid applies to click targets.
// Unlike it, keeps percent escapes, so `file=A%26B` still names the note `A&B`.
export function sanitizeUrl(url: string): string {
  if (!url) return BLANK;
  const decoded = unwrap(url, true);
  const kept = unwrap(url, false);
  if (!decoded || !kept) return BLANK;
  if (decoded[0] === '.' || decoded[0] === '/') return kept;

  const scheme = decoded.trimStart().match(RE_SCHEME);
  if (!scheme) return kept;

  const protocol = scheme[0].toLowerCase().trim();
  if (RE_INVALID_PROTOCOL.test(protocol)) return BLANK;

  const normalized = kept.trimStart().replace(/\\/g, '/');
  if (protocol === 'mailto:' || protocol.includes('://')) return normalized;

  if (protocol === 'http:' || protocol === 'https:') {
    let parsed: URL;
    try {
      parsed = new URL(normalized);
    } catch {
      return BLANK;
    }
    parsed.protocol = parsed.protocol.toLowerCase();
    parsed.hostname = parsed.hostname.toLowerCase();
    return parsed.toString();
  }
  return normalized;
}

// A URL as written in a diagram: entity codes are read first, so they cannot hide a scheme.
export function safeUrl(url: string): string {
  return sanitizeUrl(decodeEntities(url));
}

const LINK_SCHEMES = ['http', 'https', 'mailto', 'tel'];
const IMAGE_SCHEMES = ['http', 'https'];
const RE_SCHEME_NAME = /^([a-z][a-z0-9+.-]*):/i;
const RE_NETWORK_PATH = /^[\\/]{2}/;

// Stricter than Mermaid, which lets any scheme but three through: a diagram in a shared note
// could otherwise open a local file, a network share, or another application.
function allowed(url: string, schemes: readonly string[]): boolean {
  if (RE_NETWORK_PATH.test(url)) return false;
  const scheme = RE_SCHEME_NAME.exec(url);
  return !scheme || schemes.includes(scheme[1].toLowerCase());
}

// The decoded URL is checked too, for an app that decodes the href before opening it.
function allow(url: string, schemes: readonly string[]): string | undefined {
  const decoded = unwrap(url, true);
  return decoded !== undefined && allowed(url, schemes) && allowed(decoded, schemes) ? url : undefined;
}

// Both take a URL that sanitizeUrl has passed, and keep it only when it is relative or its
// scheme is one the host allows. One that is refused is left out of the drawing: written as
// `about:blank`, a link would still take the page it is in somewhere when clicked.
export function linkUrl(url: string, options: RenderOptions): string | undefined {
  return options.links === false ? undefined : allow(url, options.linkSchemes ?? LINK_SCHEMES);
}

export function imageUrl(url: string, options: RenderOptions): string | undefined {
  return options.images === false ? undefined : allow(url, options.imageSchemes ?? IMAGE_SCHEMES);
}

const RE_DROPPED = new RegExp(RE_INVALID_XML.source, 'gu');

// The href keeps the original name, so sanitizing alone is not enough.
export function internalLink(name: string, options: RenderOptions): boolean {
  const url = sanitizeUrl(name);
  if (url === BLANK || linkUrl(url, options) === undefined) return false;
  // Match character removal by esc() and the browser.
  const seen = name
    .replace(RE_DROPPED, '')
    .replace(/[\t\n\r]/g, '')
    .replace(/^[\x00-\x20]+/, '');
  return linkUrl(seen, options) !== undefined;
}

export function linked(
  body: string,
  url: string,
  id: string,
  links: LinkInfo[],
  options: RenderOptions,
  attrs = ''
): string {
  const href = linkUrl(url, options);
  if (href === undefined) return body;
  links.push({ id, href, internal: false });
  return `<a href="${esc(href)}"${attrs}${relAttr(options)}>${body}</a>`;
}

export function relAttr(options: RenderOptions): string {
  const rel = options.linkRel ?? 'noopener';
  return rel ? ` rel="${esc(rel)}"` : '';
}
