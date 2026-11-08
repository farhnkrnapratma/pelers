// Checks that a string is well-formed XML: balanced tags, quoted attributes, escaped text.
export function assertWellFormed(xml: string): void {
  const bad = /[\x00-\x08\x0b\x0c\x0e-\x1f\x7f\ud800-\udfff\ufffe\uffff]/u.exec(xml);
  if (bad)
    throw new Error(
      `Character U+${bad[0].charCodeAt(0).toString(16).toUpperCase()} near ${JSON.stringify(xml.slice(Math.max(0, bad.index - 20), bad.index + 20))}`
    );
  const stack: string[] = [];
  const re = /<(\/?)([A-Za-z][\w:.-]*)((?:\s+[\w:.-]+="[^"<]*")*)\s*(\/?)>/y;
  let i = 0;
  while (i < xml.length) {
    const lt = xml.indexOf('<', i);
    const text = xml.slice(i, lt === -1 ? xml.length : lt);
    if (text.includes('>')) throw new Error(`Unescaped ">" in text near ${JSON.stringify(text.slice(0, 40))}`);
    if (/&(?!(?:amp|lt|gt|quot|apos|#\d+|#x[0-9a-fA-F]+);)/.test(text)) {
      throw new Error(`Unescaped "&" in text near ${JSON.stringify(text.slice(0, 40))}`);
    }
    if (lt === -1) break;
    re.lastIndex = lt;
    const m = re.exec(xml);
    if (!m) throw new Error(`Malformed tag at ${lt}: ${JSON.stringify(xml.slice(lt, lt + 60))}`);
    const [, closing, name, attrs, selfClosing] = m;
    if (/&(?!(?:amp|lt|gt|quot|apos|#\d+|#x[0-9a-fA-F]+);)/.test(attrs)) {
      throw new Error(`Unescaped "&" in attributes of <${name}>`);
    }
    const seen = new Set<string>();
    for (const a of attrs.matchAll(/([^\s=]+)="[^"]*"/g)) {
      if (seen.has(a[1])) throw new Error(`<${name}> has ${a[1]} twice`);
      seen.add(a[1]);
    }
    if (closing) {
      const open = stack.pop();
      if (open !== name) throw new Error(`</${name}> closes <${open}>`);
    } else if (!selfClosing) {
      stack.push(name);
    }
    i = re.lastIndex;
  }
  if (stack.length > 0) throw new Error(`Unclosed <${stack[stack.length - 1]}>`);
}

export interface XmlElement {
  name: string;
  attrs: Map<string, string>;
}

// An attribute value as a browser reads it, with character references turned into their characters.
function decode(value: string): string {
  return value
    .replace(/&#x([0-9a-fA-F]+);/g, (_m, hex: string) => String.fromCodePoint(Math.min(parseInt(hex, 16), 0x10ffff)))
    .replace(/&#(\d+);/g, (_m, dec: string) => String.fromCodePoint(Math.min(Number(dec), 0x10ffff)))
    .replace(/&quot;/g, '"')
    .replace(/&apos;/g, "'")
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&amp;/g, '&');
}

// Lists every element with its attributes. Call assertWellFormed first.
export function elements(xml: string): XmlElement[] {
  const out: XmlElement[] = [];
  const tag = /<([A-Za-z][\w:.-]*)((?:\s+[\w:.-]+="[^"<]*")*)\s*\/?>/g;
  const attr = /([\w:.-]+)="([^"<]*)"/g;
  let m: RegExpExecArray | null;
  while ((m = tag.exec(xml)) !== null) {
    const attrs = new Map<string, string>();
    let a: RegExpExecArray | null;
    attr.lastIndex = 0;
    while ((a = attr.exec(m[2])) !== null) {
      attrs.set(a[1], a[2].includes('&') ? decode(a[2]) : a[2]);
    }
    out.push({ name: m[1], attrs });
  }
  return out;
}
