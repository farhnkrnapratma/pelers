// The window an element is in, which is not the script's own when the element is in a popup.
// Styles and resizes are asked of that window: another window's answers can be late or absent.
export function windowOf(element: Element): typeof globalThis {
  return element.ownerDocument?.defaultView ?? globalThis;
}

// The width inside an element's padding, or 0 when it has none yet.
export function room(element: HTMLElement, style: CSSStyleDeclaration): number {
  const width = element.clientWidth - (parseFloat(style.paddingLeft) || 0) - (parseFloat(style.paddingRight) || 0);
  return width > 0 ? width : 0;
}
