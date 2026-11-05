// Mermaid asks the browser whether the first word of a box title is a color. Pele has no browser,
// so it knows the color keywords and the rgb() and hsl() notations. Other functions, such as calc()
// and var(), are not recognized.
const KEYWORDS =
  'aliceblue antiquewhite aqua aquamarine azure beige bisque black blanchedalmond blue blueviolet brown burlywood ' +
  'cadetblue chartreuse chocolate coral cornflowerblue cornsilk crimson cyan darkblue darkcyan darkgoldenrod darkgray ' +
  'darkgreen darkgrey darkkhaki darkmagenta darkolivegreen darkorange darkorchid darkred darksalmon darkseagreen ' +
  'darkslateblue darkslategray darkslategrey darkturquoise darkviolet deeppink deepskyblue dimgray dimgrey dodgerblue ' +
  'firebrick floralwhite forestgreen fuchsia gainsboro ghostwhite gold goldenrod gray green greenyellow grey honeydew ' +
  'hotpink indianred indigo ivory khaki lavender lavenderblush lawngreen lemonchiffon lightblue lightcoral lightcyan ' +
  'lightgoldenrodyellow lightgray lightgreen lightgrey lightpink lightsalmon lightseagreen lightskyblue lightslategray ' +
  'lightslategrey lightsteelblue lightyellow lime limegreen linen magenta maroon mediumaquamarine mediumblue ' +
  'mediumorchid mediumpurple mediumseagreen mediumslateblue mediumspringgreen mediumturquoise mediumvioletred ' +
  'midnightblue mintcream mistyrose moccasin navajowhite navy oldlace olive olivedrab orange orangered orchid ' +
  'palegoldenrod palegreen paleturquoise palevioletred papayawhip peachpuff peru pink plum powderblue purple ' +
  'rebeccapurple red rosybrown royalblue saddlebrown salmon sandybrown seagreen seashell sienna silver skyblue ' +
  'slateblue slategray slategrey snow springgreen steelblue tan teal thistle tomato turquoise violet wheat white ' +
  'whitesmoke yellow yellowgreen transparent currentcolor inherit initial unset revert ' +
  'accentcolor accentcolortext activetext buttonborder buttonface buttontext canvas canvastext field fieldtext ' +
  'graytext highlight highlighttext linktext mark marktext selecteditem selecteditemtext visitedtext ' +
  'activeborder activecaption appworkspace background buttonhighlight buttonshadow captiontext inactiveborder ' +
  'inactivecaption inactivecaptiontext infobackground infotext menu menutext scrollbar threeddarkshadow threedface ' +
  'threedhighlight threedlightshadow threedshadow window windowframe windowtext';

let keywords: Set<string> | undefined;

const RE_TERM = /^[+-]?(?:\d+(?:\.\d+)?|\.\d+)(?:e[+-]?\d+)?(%|deg|grad|rad|turn)?$/i;

const enum K {
  Bad,
  Number,
  Percent,
  Angle,
  None,
}

function kind(term: string): K {
  if (term.toLowerCase() === 'none') return K.None;
  const m = RE_TERM.exec(term);
  if (!m) return K.Bad;
  return m[1] === undefined ? K.Number : m[1] === '%' ? K.Percent : K.Angle;
}

function isAlpha(term: string, modern: boolean): boolean {
  const k = kind(term);
  return k === K.Number || k === K.Percent || (modern && k === K.None);
}

function isFunction(color: string): boolean {
  const hsl = color.startsWith('hsl');
  const open = color.indexOf('(');
  const name = color.slice(0, open);
  if (name !== 'rgb' && name !== 'rgba' && name !== 'hsl' && name !== 'hsla') return false;
  if (color.length > 120 || color.indexOf(')') !== color.length - 1) return false;
  const inner = color.slice(open + 1, -1).trim();
  let parts: string[];
  let alpha: string | undefined;
  const legacy = inner.includes(',');
  if (legacy) {
    parts = inner.split(',').map((s) => s.trim());
    if (parts.length === 4) alpha = parts.pop();
  } else {
    const slash = inner.split('/');
    if (slash.length > 2) return false;
    parts = slash[0].trim().split(/\s+/);
    if (slash.length === 2) alpha = slash[1].trim();
  }
  if (parts.length !== 3 || (alpha !== undefined && !isAlpha(alpha, !legacy))) return false;
  const kinds = parts.map(kind);
  if (legacy) {
    if (hsl) return (kinds[0] === K.Number || kinds[0] === K.Angle) && kinds[1] === K.Percent && kinds[2] === K.Percent;
    return (kinds[0] === K.Number || kinds[0] === K.Percent) && kinds[1] === kinds[0] && kinds[2] === kinds[0];
  }
  for (let i = 0; i < 3; i++) {
    const k = kinds[i];
    const ok = k === K.Number || k === K.None || (hsl && i === 0 ? k === K.Angle : k === K.Percent);
    if (!ok) return false;
  }
  return true;
}

export function isCssColor(color: string): boolean {
  if (color.includes('(')) return isFunction(color);
  keywords ??= new Set(KEYWORDS.split(' '));
  return keywords.has(color.toLowerCase());
}
