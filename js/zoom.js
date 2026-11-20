import { windowOf } from "./window.js";
export function frame(natural, box, scale, x, y) {
  const width = box.width / scale;
  const height = box.height / scale;
  const within = (at, view, whole) => view >= whole ? Math.min(Math.max(at, whole - view), 0) : Math.min(Math.max(at, 0), whole - view);
  return { x: within(x, width, natural.width), y: within(y, height, natural.height), width, height };
}
const STEP = 1.25;
const PAN = 48;
const GAP = 4;
const BAR_STYLE = "--_bg:var(--pelers-bg,#fff);--_fg:var(--pelers-fg,#1f1f1f);--_b:var(--pelers-border,#8a8a8a);" + "position:absolute;top:var(--pelers-zoom-top,8px);right:var(--pelers-zoom-right,8px);" + "bottom:var(--pelers-zoom-bottom,auto);left:var(--pelers-zoom-left,auto);display:flex;flex-direction:column;gap:2px;padding:2px;" + "width:auto;max-width:none;border:1px solid var(--_b);border-radius:var(--pelers-radius,4px);background:var(--_bg);color:var(--_fg);line-height:0";
const BUTTON_STYLE = "appearance:none;display:grid;place-items:center;width:24px;height:24px;margin:0;padding:0;border:0;" + "border-radius:calc(var(--pelers-radius,4px) - 1px);background:transparent;color:inherit;font:inherit;cursor:var(--pelers-zoom-cursor,pointer)";
const ICONS = {
  zoomIn: "M5 12h14M12 5v14",
  zoomOut: "M5 12h14",
  reset: "M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8M3 3v5h5"
};
const LABELS = { zoomIn: "Zoom in", zoomOut: "Zoom out", reset: "Reset zoom" };
export function enableZoom(element, options = {}) {
  return zoomWith(element, options, true);
}
export function zoomWith(element, options, observe) {
  let svg = null;
  let natural = { width: 0, height: 0 };
  let rest = { width: 0, height: 0 };
  let box = { width: 0, height: 0 };
  let roomy = false;
  let bleed = "";
  let restStyle = { width: "", height: "", maxWidth: "", margin: "", display: "" };
  let cleared = null;
  let scale = 0;
  let x = 0;
  let y = 0;
  let watched = 0;
  const view = () => windowOf(element);
  const fit = () => natural.width > 0 && rest.width > 0 ? rest.width / natural.width : 1;
  const most = () => Math.max(fit(), options.maxScale ?? 3);
  const zoomable = () => svg !== null && natural.width > 0 && (options.always === true || fit() < 0.999);
  const measure = () => {
    if (!svg || scale !== 0)
      return;
    const at = svg.getBoundingClientRect();
    rest = { width: at.width, height: at.height };
    box = { width: Math.max(rest.width, element.clientWidth), height: Math.max(rest.height, element.clientHeight) };
    roomy = box.width > rest.width + 1 || box.height > rest.height + 1;
    if (!roomy)
      box = rest;
    const style = view().getComputedStyle(element);
    bleed = ["Top", "Right", "Bottom", "Left"].map((side) => `${-(parseFloat(style.getPropertyValue(`padding-${side.toLowerCase()}`)) || 0)}px`).join(" ");
  };
  let bar;
  const buttons = {};
  let positioned = false;
  const offer = () => {
    if (options.controls === false)
      return;
    const can = zoomable();
    if (!bar) {
      if (!can)
        return;
      const doc = element.ownerDocument;
      bar = doc.createElement("div");
      bar.className = "pelers pelers-zoom";
      bar.setAttribute("style", BAR_STYLE);
      const act = { zoomIn: () => show((scale || fit()) * STEP * STEP), zoomOut: () => show((scale || fit()) / (STEP * STEP)), reset: () => settle() };
      for (const name of ["zoomIn", "zoomOut", "reset"]) {
        const button = doc.createElement("button");
        const label = options.labels?.[name] ?? LABELS[name];
        button.type = "button";
        button.className = `pelers-zoom-${name === "zoomIn" ? "in" : name === "zoomOut" ? "out" : "reset"}`;
        button.setAttribute("style", BUTTON_STYLE);
        button.setAttribute("aria-label", label);
        button.title = label;
        button.innerHTML = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="${ICONS[name]}"/></svg>`;
        button.addEventListener("click", act[name]);
        buttons[name] = button;
        bar.append(button);
      }
    }
    if (can && !positioned && view().getComputedStyle(element).position === "static") {
      element.style.position = "relative";
      positioned = true;
    }
    if (can && bar.parentNode !== element)
      element.append(bar);
    bar.hidden = !can;
    bar.style.display = can ? "flex" : "none";
    const dim = (button, off) => {
      if (!button)
        return;
      button.disabled = off;
      button.style.opacity = off ? "0.35" : "";
      button.style.cursor = off ? "default" : "var(--pelers-zoom-cursor,pointer)";
    };
    dim(buttons.zoomIn, (scale || fit()) >= most() * 0.999);
    dim(buttons.zoomOut, scale === 0);
    if (buttons.reset)
      buttons.reset.style.display = scale === 0 ? "none" : "grid";
  };
  const ours = (event) => bar !== undefined && event.target instanceof view().Node && bar.contains(event.target);
  const restore = () => {
    if (!svg)
      return;
    unclear();
    svg.style.width = restStyle.width;
    svg.style.height = restStyle.height;
    svg.style.maxWidth = restStyle.maxWidth;
    svg.style.margin = restStyle.margin;
    svg.style.display = restStyle.display;
  };
  const settle = () => {
    if (!svg)
      return;
    restore();
    scale = 0;
    svg.setAttribute("viewBox", `0 0 ${natural.width} ${natural.height}`);
    svg.style.cursor = "";
    svg.style.userSelect = "";
    measure();
    svg.style.touchAction = zoomable() ? "pan-y" : "";
    if (zoomable())
      svg.setAttribute("tabindex", "0");
    else
      svg.removeAttribute("tabindex");
    take(zoomable());
    offer();
    clear();
  };
  const clear = () => {
    if (!svg || !bar || bar.hidden)
      return;
    const at = bar.getBoundingClientRect();
    const hidden = [...svg.querySelectorAll("text")].some((text) => {
      const r = text.getBoundingClientRect();
      return r.right > at.left && r.left < at.right && r.bottom > at.top && r.top < at.bottom;
    });
    const drawing = svg.getBoundingClientRect();
    const end = at.left - GAP;
    if (!hidden || drawing.right - end >= drawing.width / 2)
      return;
    const outer = element.getBoundingClientRect();
    const k = element.offsetWidth > 0 ? outer.width / element.offsetWidth : 1;
    const padding = parseFloat(view().getComputedStyle(element).paddingRight) || 0;
    const edge = outer.left + (element.clientLeft + element.clientWidth - padding) * k;
    const before = [svg.style.maxWidth, svg.style.marginRight];
    svg.style.maxWidth = `${(end - drawing.left) / k}px`;
    svg.style.marginRight = `${Math.max(edge - end, 0) / k}px`;
    cleared = { before, after: [svg.style.maxWidth, svg.style.marginRight] };
    if (svg.getBoundingClientRect().right > end + 1)
      unclear();
    measure();
  };
  const unclear = () => {
    if (!svg || !cleared)
      return;
    if (svg.style.maxWidth === cleared.after[0])
      svg.style.maxWidth = cleared.before[0];
    if (svg.style.marginRight === cleared.after[1])
      svg.style.marginRight = cleared.before[1];
    cleared = null;
  };
  const show = (next, clientX, clientY) => {
    if (!svg)
      return;
    if (scale === 0)
      measure();
    if (!zoomable())
      return;
    const from = scale || fit();
    const to = Math.min(Math.max(next, fit()), most());
    if (to <= fit() * 1.001) {
      settle();
      return;
    }
    const before = svg.getBoundingClientRect();
    const atX = clientX ?? before.left + before.width / 2;
    const atY = clientY ?? before.top + before.height / 2;
    const pointX = (scale ? x : 0) + (atX - before.left) / from;
    const pointY = (scale ? y : 0) + (atY - before.top) / from;
    scale = to;
    let after = before;
    if (roomy) {
      svg.style.maxWidth = "none";
      svg.style.margin = bleed;
      svg.style.display = "block";
      svg.style.width = `${box.width}px`;
      svg.style.height = `${box.height}px`;
      after = svg.getBoundingClientRect();
    }
    place(pointX - (atX - after.left) / scale, pointY - (atY - after.top) / scale);
    svg.style.cursor = "grab";
    svg.style.userSelect = "none";
    svg.style.touchAction = "none";
    offer();
  };
  const place = (toX, toY) => {
    if (!svg)
      return;
    const shown = frame(natural, box, scale, toX, toY);
    x = shown.x;
    y = shown.y;
    svg.setAttribute("viewBox", `${x} ${y} ${shown.width} ${shown.height}`);
  };
  const move = (dx, dy) => {
    if (scale !== 0)
      place(x - dx / scale, y - dy / scale);
  };
  const resized = () => {
    watch();
    const size = element.clientWidth + element.clientHeight;
    if (size === watched)
      return;
    watched = size;
    settle();
  };
  let observer;
  let observed;
  const watch = () => {
    if (!observe)
      return;
    const next = view();
    if (next === observed)
      return;
    observer?.disconnect();
    observed = next;
    observer = next.ResizeObserver ? new next.ResizeObserver(resized) : undefined;
    observer?.observe(element);
  };
  const refresh = () => {
    watch();
    if (scale !== 0)
      restore();
    else
      unclear();
    const first = element.firstElementChild;
    svg = first && first.localName === "svg" ? first : null;
    scale = 0;
    if (!svg) {
      bar?.remove();
      take(false);
      return;
    }
    natural = { width: Number(svg.getAttribute("width")) || 0, height: Number(svg.getAttribute("height")) || 0 };
    restStyle = { width: svg.style.width, height: svg.style.height, maxWidth: svg.style.maxWidth, margin: svg.style.margin, display: svg.style.display };
    settle();
  };
  const pointers = new Map;
  let dragged = false;
  let travelled = 0;
  let pinch = 0;
  const spread = () => {
    const [a, b] = [...pointers.values()];
    return { distance: Math.hypot(a.x - b.x, a.y - b.y), x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
  };
  const onPress = () => {
    dragged = false;
  };
  const onPointerDown = (event) => {
    if (!zoomable() || ours(event) || event.pointerType === "mouse" && event.button !== 0)
      return;
    pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
    travelled = 0;
    if (pointers.size === 2)
      pinch = spread().distance;
    if (pointers.size === 2 || scale !== 0) {
      element.setPointerCapture?.(event.pointerId);
      if (svg)
        svg.style.cursor = "grabbing";
    }
  };
  const onPointerMove = (event) => {
    const last = pointers.get(event.pointerId);
    if (!last)
      return;
    if (pointers.size === 2) {
      const before = spread();
      pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
      const now = spread();
      if (pinch > 0 && now.distance > 0)
        show((scale || fit()) * (now.distance / before.distance), now.x, now.y);
      move(now.x - before.x, now.y - before.y);
      dragged = true;
      return;
    }
    pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
    if (scale === 0)
      return;
    travelled += Math.abs(event.clientX - last.x) + Math.abs(event.clientY - last.y);
    if (travelled > 4)
      dragged = true;
    move(event.clientX - last.x, event.clientY - last.y);
  };
  const onPointerUp = (event) => {
    if (!pointers.delete(event.pointerId))
      return;
    pinch = 0;
    if (pointers.size === 0 && svg && scale !== 0)
      svg.style.cursor = "grab";
  };
  const onClick = (event) => {
    if (!dragged || event.detail === 0)
      return;
    dragged = false;
    event.preventDefault();
    event.stopPropagation();
  };
  const onWheel = (event) => {
    if (!(event.ctrlKey || event.metaKey) || !zoomable())
      return;
    event.preventDefault();
    if (scale === 0)
      measure();
    const turn = Math.min(Math.max(-event.deltaY, -24), 24);
    show((scale || fit()) * 2 ** (turn * 0.02), event.clientX, event.clientY);
  };
  const onDoubleClick = (event) => {
    if (!zoomable() || ours(event))
      return;
    event.preventDefault();
    if (scale !== 0)
      settle();
    else
      show(Math.max(1, fit() * 2), event.clientX, event.clientY);
  };
  const onKeyDown = (event) => {
    if (!zoomable() || ours(event) || event.ctrlKey || event.metaKey || event.altKey)
      return;
    const key = event.key;
    if (key === "+" || key === "=")
      show((scale || fit()) * STEP);
    else if (key === "-")
      show((scale || fit()) / STEP);
    else if (scale === 0)
      return;
    else if (key === "0" || key === "Escape")
      settle();
    else if (key === "ArrowLeft")
      move(PAN, 0);
    else if (key === "ArrowRight")
      move(-PAN, 0);
    else if (key === "ArrowUp")
      move(0, PAN);
    else if (key === "ArrowDown")
      move(0, -PAN);
    else
      return;
    event.preventDefault();
  };
  const onFocus = (event) => {
    if (!svg || event.target !== svg)
      return;
    let visible = true;
    try {
      visible = svg.matches(":focus-visible");
    } catch {}
    svg.style.outline = visible ? "" : "none";
  };
  let gesture = 0;
  const onGestureStart = (event) => {
    if (!zoomable())
      return;
    event.preventDefault();
    if (scale === 0)
      measure();
    gesture = scale || fit();
  };
  const onGestureChange = (event) => {
    if (gesture === 0)
      return;
    event.preventDefault();
    const { scale: by, clientX, clientY } = event;
    show(gesture * by, clientX, clientY);
  };
  const onGestureEnd = () => {
    gesture = 0;
  };
  const listeners = [
    ["pointerdown", onPress, { capture: true }],
    ["pointerdown", onPointerDown],
    ["pointermove", onPointerMove],
    ["pointerup", onPointerUp],
    ["pointercancel", onPointerUp],
    ["click", onClick, { capture: true }],
    ["dblclick", onDoubleClick],
    ["keydown", onKeyDown],
    ["focusin", onFocus]
  ];
  for (const [type, listener, how] of listeners)
    element.addEventListener(type, listener, how);
  const gestures = [
    ["wheel", onWheel, { passive: false }],
    ["gesturestart", onGestureStart, { passive: false }],
    ["gesturechange", onGestureChange, { passive: false }],
    ["gestureend", onGestureEnd, { passive: false }]
  ];
  let taking = false;
  function take(on) {
    if (on === taking)
      return;
    taking = on;
    for (const [type, listener, how] of gestures) {
      if (on)
        element.addEventListener(type, listener, how);
      else
        element.removeEventListener(type, listener, how);
    }
    if (!on)
      gesture = 0;
  }
  watched = element.clientWidth + element.clientHeight;
  refresh();
  return {
    get scale() {
      return scale || fit();
    },
    get fit() {
      return fit();
    },
    zoomBy(factor) {
      show((scale || fit()) * factor);
    },
    zoomTo(next) {
      show(next);
    },
    reset: settle,
    refresh,
    resized,
    destroy() {
      settle();
      unclear();
      if (svg) {
        svg.style.touchAction = "";
        svg.removeAttribute("tabindex");
      }
      bar?.remove();
      if (positioned)
        element.style.position = "";
      observer?.disconnect();
      take(false);
      for (const [type, listener, how] of listeners)
        element.removeEventListener(type, listener, how);
    }
  };
}
