import { isLineEnd } from '../../util/chars.js';
import { parseGenericTypes } from '../common/generics.js';

export type Visibility = '#' | '+' | '~' | '-' | '';

function isVisibility(ch: string): ch is Visibility {
  return ch === '#' || ch === '+' || ch === '~' || ch === '-';
}

export class ClassMember {
  id = '';
  memberType: 'method' | 'attribute';
  visibility: Visibility = '';
  classifier = '';
  parameters = '';
  returnType = '';

  constructor(input: string, memberType: 'method' | 'attribute') {
    this.memberType = memberType;
    // Mermaid throws a TypeError for a method with no opening parenthesis, such as `run)`.
    // It is kept here as an attribute, so it is shown as written.
    if (memberType !== 'method' || !this.parseMethod(input)) this.parseAttribute(input);
    this.id = this.id.startsWith(' ') ? ' ' + this.id.trim() : this.id.trim();
  }

  // The same split as Mermaid's /([#+~-])?(.+)\((.*)\)([\s$*])?(.*)([$*])?/, found in one pass:
  // the first line with an opening parenthesis after its first character and a closing one later.
  private parseMethod(input: string): boolean {
    const n = input.length;
    for (let start = 0; start < n;) {
      let end = start;
      while (end < n && !isLineEnd(input.charCodeAt(end))) end++;
      let close = end - 1;
      while (close > start && input.charCodeAt(close) !== 41) close--;
      let open = close - 1;
      while (open > start && input.charCodeAt(open) !== 40) open--;
      if (open > start) {
        const first = input[start];
        const visible = isVisibility(first) && open > start + 1;
        if (visible) this.visibility = first;
        this.id = input.slice(visible ? start + 1 : start, open);
        this.parameters = input.slice(open + 1, close).trim();
        let rest = close + 1;
        let classifier = '';
        if (rest < n) {
          const next = input[rest];
          if (next === '$' || next === '*' || next.trim() === '') {
            classifier = next.trim();
            rest++;
          }
        }
        let stop = rest;
        while (stop < n && !isLineEnd(input.charCodeAt(stop))) stop++;
        this.returnType = input.slice(rest, stop).trim();
        if (classifier === '') {
          const last = this.returnType.slice(-1);
          if (last === '$' || last === '*') {
            classifier = last;
            this.returnType = this.returnType.slice(0, -1);
          }
        }
        this.classifier = classifier;
        return true;
      }
      start = end + 1;
    }
    this.memberType = 'attribute';
    return false;
  }

  private parseAttribute(input: string): void {
    const first = input.slice(0, 1);
    const last = input.slice(-1);
    if (isVisibility(first)) this.visibility = first;
    if (last === '$' || last === '*') this.classifier = last;
    this.id = input.substring(this.visibility === '' ? 0 : 1, this.classifier === '' ? input.length : input.length - 1);
  }

  getDisplayDetails(): { displayText: string; cssStyle: string } {
    let displayText = this.visibility + parseGenericTypes(this.id);
    if (this.memberType === 'method') {
      displayText += `(${parseGenericTypes(this.parameters.trim())})`;
      if (this.returnType) displayText += ' : ' + parseGenericTypes(this.returnType);
    }
    return { displayText: displayText.trim(), cssStyle: this.parseClassifier() };
  }

  parseClassifier(): string {
    return this.classifier === '*' ? 'font-style:italic;' : this.classifier === '$' ? 'text-decoration:underline;' : '';
  }
}
