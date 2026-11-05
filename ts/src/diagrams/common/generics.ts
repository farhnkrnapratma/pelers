function tildes(text: string): number {
  let count = 0;
  for (let i = text.indexOf('~'); i !== -1; i = text.indexOf('~', i + 1)) count++;
  return count;
}

// Pairs tildes from the outside in: the first with the last, the second with the one before it.
function pairTildes(text: string): string {
  const count = tildes(text);
  if (count <= 1) return text;
  const lead = count % 2 !== 0 && text.startsWith('~');
  const chars = text.split('');
  const at: number[] = [];
  for (let i = lead ? 1 : 0; i < chars.length; i++) if (chars[i] === '~') at.push(i);
  for (let lo = 0, hi = at.length - 1; lo < hi; lo++, hi--) {
    chars[at[lo]] = '<';
    chars[at[hi]] = '>';
  }
  return chars.join('');
}

// Turns `List~int~` into `List<int>`, with the quirks of Mermaid's parseGenericTypes around commas.
export function parseGenericTypes(input: string): string {
  if (!input.includes('~')) return input;
  const sets = input.split(/(,)/);
  const output: string[] = [];
  for (let i = 0; i < sets.length; i++) {
    let set = sets[i];
    if (set === ',' && i > 0 && i + 1 < sets.length && tildes(sets[i - 1]) === 1 && tildes(sets[i + 1]) === 1) {
      set = sets[i - 1] + ',' + sets[i + 1];
      i++;
      output.pop();
    }
    output.push(pairTildes(set));
  }
  return output.join('');
}
