// Whether `word`, written in lower case, is at `at`. Only ASCII letters match in either case,
// as with the `i` flag on a regular expression without the `u` flag.
export function wordAt(src: string, at: number, word: string): boolean {
  for (let k = 0; k < word.length; k++) {
    const c = src.charCodeAt(at + k);
    const w = word.charCodeAt(k);
    if (c !== w && !(w >= 97 && w <= 122 && c === w - 32)) return false;
  }
  return true;
}
