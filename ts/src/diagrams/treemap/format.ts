// The parts of d3-format's specifier that treemap values use: an optional `$`, `,` for thousands
// separators, a precision, and a type of `f`, `%`, `d`, `e` or none. Fill, alignment and width are
// read and ignored. Anything else falls back to the default, `,`.
const RE_SPEC = /^(?:.?[<>=^])?([-+( ])?(\$)?0?\d*(,)?(?:\.(\d+))?(~)?([a-zA-Z%])?$/;

function group(digits: string): string {
  let out = '';
  for (let i = digits.length; i > 0; i -= 3) out = digits.slice(Math.max(0, i - 3), i) + (out ? ',' + out : '');
  return out;
}

export function formatter(spec: string): (value: number) => string {
  // Mermaid documents `$0,0` for a dollar amount with thousands separators.
  const m = RE_SPEC.exec(spec === '$0,0' ? '$,' : spec) ?? RE_SPEC.exec(',')!;
  const plus = m[1] === '+';
  const comma = m[3] !== undefined;
  const precision = m[4] === undefined ? undefined : Math.min(Number(m[4]), 20);
  const type = m[6] ?? '';

  return (value) => {
    const abs = Math.abs(value);
    let body: string;
    let suffix = '';
    if (type === 'f') body = abs.toFixed(precision ?? 6);
    else if (type === '%') {
      body = (abs * 100).toFixed(precision ?? 6);
      suffix = '%';
    } else if (type === 'd') body = Math.round(abs).toFixed(0);
    else if (type === 'e') body = abs.toExponential(precision ?? 6);
    else body = String(Number(abs.toPrecision(Math.max(1, precision ?? 12))));

    if (m[5] && body.includes('.') && !body.includes('e')) body = body.replace(/\.?0+$/, '');
    if (comma && !body.includes('e')) {
      const dot = body.indexOf('.');
      body = dot === -1 ? group(body) : group(body.slice(0, dot)) + body.slice(dot);
    }
    return (value < 0 ? '-' : plus ? '+' : '') + (m[2] ?? '') + body + suffix;
  };
}
