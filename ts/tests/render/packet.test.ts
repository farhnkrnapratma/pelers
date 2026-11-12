import { describe, expect, it } from 'vitest';
import { parse, render, supports } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { elements } from '../support/xml.js';

const options = { measurer: metricsMeasurer };

const TCP = 'packet\n0-15: "Source Port"\n16-31: "Destination Port"\n32-63: "Sequence Number"\n64: "URG"';

function config(packet: string, body: string): string {
  return `---\nconfig:\n  packet:\n${packet}\n---\n${body}`;
}

// The x position of each field's box, by its bit range.
function boxes(svg: string): Map<string, number> {
  const out = new Map<string, number>();
  let id = '';
  for (const el of elements(svg)) {
    if (el.name === 'g' && el.attrs.has('data-id')) id = el.attrs.get('data-id')!;
    if (el.name === 'rect') out.set(id, Number(el.attrs.get('x')));
  }
  return out;
}

describe('packet rendering', () => {
  it('draws a box and a label per field, in rows of 32 bits', () => {
    const { svg, type, width } = render(TCP, options);
    expect(type).toBe('packet');
    expect(supports('packet-beta\n0: "a"')).toBe(true);
    expect(svg.match(/class="pelers-node"/g)?.length).toBe(4);
    expect(svg).toContain('>Source Port<');
    expect(svg).toContain('data-id="32-63"');
    expect(width).toBe(32 * 32 - 5 + 16);
    const ys = new Set(
      elements(svg)
        .filter((el) => el.name === 'rect')
        .map((el) => el.attrs.get('y'))
    );
    expect(ys.size).toBe(3);
  });

  it('numbers the first and last bit of each field, and a single bit once', () => {
    const { svg } = render('packet\n0-15: "a"\n16: "b"', options);
    const bits = svg.slice(svg.indexOf('pelers-packet-bits'));
    expect(bits.match(/<text/g)?.length).toBe(3);
    expect(bits).toContain('>0<');
    expect(bits).toContain('text-anchor="end">15<');
    expect(bits).toContain('text-anchor="middle">16<');
  });

  it('hides bit numbers when showBits is off, and takes less height', () => {
    const shown = render('packet\n0-15: "a"\n32-47: "b"'.replace('32-47', '16-47'), options);
    const hidden = render(config('    showBits: false', 'packet\n0-15: "a"\n16-47: "b"'), options);
    expect(hidden.svg).not.toContain('pelers-packet-bits');
    expect(hidden.height).toBeLessThan(shown.height);
  });

  it('repeats a field on every row it spans', () => {
    const { svg } = render('packet\n0-10: "test"\n11-90: "multiple"', options);
    expect(svg.match(/>multiple</g)?.length).toBe(3);
    expect(svg).toContain('data-id="11-31"');
    expect(svg).toContain('data-id="32-63"');
    expect(svg).toContain('data-id="64-90"');
  });

  it('honours bitsPerRow, bitWidth, rowHeight, paddingX and paddingY', () => {
    const body = 'packet\n0-7: "a"\n8-15: "b"\n16-23: "c"';
    const base = render(body, options);
    const narrow = render(config('    bitsPerRow: 8\n    bitWidth: 10', body), options);
    expect(narrow.width).toBe(8 * 10 - 5 + 16);
    expect(narrow.height).toBeGreaterThan(base.height);
    const tall = render(config('    rowHeight: 80', body), options);
    expect(tall.height).toBe(base.height + 48);
    const loose = render(config('    bitsPerRow: 8\n    paddingY: 30', body), options);
    const tight = render(config('    bitsPerRow: 8\n    paddingY: 0', body), options);
    expect(loose.height - tight.height).toBe(60);
    const gap = boxes(render(config('    paddingX: 20', body), options).svg);
    const rects = elements(render(config('    paddingX: 20', body), options).svg).filter((el) => el.name === 'rect');
    expect(Number(rects[0].attrs.get('width'))).toBe(8 * 32 - 20);
    expect(gap.get('8-15')! - gap.get('0-7')!).toBe(8 * 32);
  });

  it('mirrors each row when bitOrder is descending', () => {
    const body = 'packet\n0-7: "DATA"\n8-11: "TYPE"\n12: "EN"\n13-15: "RESERVED"\n16-19: "NEXT"';
    const ascending = boxes(render(config('    bitsPerRow: 16', body), options).svg);
    const { svg } = render(config('    bitsPerRow: 16\n    bitOrder: descending', body), options);
    const descending = boxes(svg);
    expect(ascending.get('0-7')!).toBeLessThan(ascending.get('8-11')!);
    expect(descending.get('0-7')!).toBeGreaterThan(descending.get('8-11')!);
    expect(descending.get('13-15')!).toBe(ascending.get('0-7')!);
    // A partly filled row keeps its lowest bit against the right edge.
    expect(descending.get('16-19')!).toBe(ascending.get('0-7')! + 12 * 32);
    const bits = svg.slice(svg.indexOf('pelers-packet-bits'));
    expect(bits.indexOf('>7<')).toBeLessThan(bits.indexOf('>0<'));
    expect(bits).toContain('text-anchor="end">0<');
  });

  it('shrinks or shortens a label that is wider than its field', () => {
    const { svg } = render('packet\n0: "Wide"\n1: "Extraordinarily long"\n2-31: "x"', options);
    expect(svg).toMatch(/font-size="\d+">Wide</);
    expect(svg).toMatch(/>Extr…</);
    expect(svg).not.toContain('Extraordinarily');
  });

  it('puts the title under the fields, from the diagram or the front matter', () => {
    const own = render('---\ntitle: From front matter\n---\npacket\ntitle Own\n0-31: "a"', options);
    expect(own.svg).toContain('>Own<');
    expect(own.svg).not.toContain('From front matter');
    expect(render('---\ntitle: From front matter\n---\npacket\n0-31: "a"', options).svg).toContain(
      '>From front matter<'
    );
    const all = elements(own.svg);
    const rect = all.find((el) => el.name === 'rect')!;
    const title = all[all.findIndex((el) => el.attrs.get('class') === 'pelers-title') + 1];
    expect(title.name).toBe('tspan');
    expect(Number(title.attrs.get('y'))).toBeGreaterThan(Number(rect.attrs.get('y')) + 32);
  });

  it('renders an empty packet and one with only a title', () => {
    expect(render('packet', options).svg).toContain('<svg');
    const titled = render('packet\ntitle Nothing yet', options);
    expect(titled.svg).toContain('>Nothing yet<');
    expect(titled.width).toBeLessThan(300);
  });

  it("rejects what Mermaid rejects, with Mermaid's messages", () => {
    expect(() => render('packet\n0-16: "a"\n18-20: "b"', options)).toThrow(
      'Packet block 18 - 20 is not contiguous. It should start from 17.'
    );
    expect(() => render('packet\n0-16: "a"\n25-20: "b"', options)).toThrow(
      'Packet block 25 - 20 is invalid. End must be greater than start.'
    );
    expect(() => render('packet\n+0: "a"', options)).toThrow(
      'Packet block 0 is invalid. Cannot have a zero bit field.'
    );
  });

  it('exposes the model', () => {
    const model = parse(config('    bitsPerRow: 8', 'packet\ntitle T\naccTitle: A\naccDescr: D\n+4: "a"\n+8: "b"'));
    expect(model.type).toBe('packet');
    if (model.type === 'packet') {
      expect(model.title).toBe('T');
      expect(model.accTitle).toBe('A');
      expect(model.bitsPerRow).toBe(8);
      expect(model.rows.map((row) => row.map((b) => [b.start, b.end, b.label]))).toEqual([
        [
          [0, 3, 'a'],
          [4, 7, 'b'],
        ],
        [[8, 11, 'b']],
      ]);
    }
  });

  it('stops after ten thousand rows, as Mermaid does', () => {
    const model = parse(config('    bitsPerRow: 1', 'packet\n+20000: "a"\n+1: "b"'));
    if (model.type === 'packet') expect(model.rows.length).toBe(10000);
  });

  it('writes accessible names and ignores unusable config', () => {
    const { svg } = render('packet\naccTitle: Name\naccDescr: Text\n0-31: "a"', options);
    expect(svg).toContain('<title id="pelers-title">Name</title>');
    expect(svg).toContain('<desc id="pelers-desc">Text</desc>');
    const odd = render(
      config('    bitsPerRow: -4\n    bitWidth: "wide"\n    rowHeight: 0\n    paddingX: 9999', TCP),
      options
    );
    expect(odd.svg).not.toMatch(/NaN|Infinity|width="-/);
  });
});
