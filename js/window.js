export function windowOf(element) {
  return element.ownerDocument?.defaultView ?? globalThis;
}
export function room(element, style) {
  const width = element.clientWidth - (parseFloat(style.paddingLeft) || 0) - (parseFloat(style.paddingRight) || 0);
  return width > 0 ? width : 0;
}
