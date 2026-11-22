import { forgetTextWidths } from "./measure.js";
import { room, windowOf } from "./window.js";
import { zoomWith } from "./zoom.js";
function fontsFor(style, drawing) {
  const own = drawing ? windowOf(drawing).getComputedStyle(drawing) : style;
  return {
    family:
      own.getPropertyValue("--pelers-font").trim() ||
      style.fontFamily ||
      undefined,
    mono: own.getPropertyValue("--pelers-font-mono").trim() || undefined,
  };
}
function fontSets(element) {
  const sets = new Set([
    element.ownerDocument?.fonts,
    globalThis.document?.fonts,
  ]);
  return [...sets].filter((fonts) => fonts !== undefined);
}
const settling = new WeakMap();
function settled(fonts) {
  let done = settling.get(fonts);
  if (!done) {
    done = fonts.ready.then(() => {
      settling.delete(fonts);
      forgetTextWidths();
    });
    settling.set(fonts, done);
  }
  return done;
}
const onFontsLoaded = new WeakMap();
function afterFonts(loading, diagram) {
  Promise.all(loading.map(settled)).then(() => {
    const handle = diagram.deref();
    if (handle) onFontsLoaded.get(handle)?.();
  });
}
const mounts = new WeakMap();
let mounted = 0;
const copy = Math.random().toString(36).slice(2, 7);
const OVER_STYLE =
  "position:relative;height:1.3em;margin-top:-1.3em;line-height:1.3;font-size:0.8em;" +
  "color:var(--pelers-error,#e45756);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;";
const ALONE_STYLE =
  "margin:0;font-size:0.8em;font-family:var(--pelers-font-mono,monospace);" +
  "color:var(--pelers-error,#e45756);white-space:pre-wrap;";
function messageOf(error) {
  return error instanceof Error ? error.message : String(error);
}
function summary(message) {
  const lines = message
    .split(`
`)
    .filter((line) => line.trim() !== "");
  return lines.length > 1
    ? `${lines[0]} ${lines[lines.length - 1]}`
    : (lines[0] ?? "");
}
export function mountWith(engine, element, text, options = {}) {
  mounts.get(element)?.destroy();
  let source = text;
  let settings = options;
  let model;
  let drawnModel;
  let current = false;
  let error;
  let natural;
  let shown;
  const idPrefix = `pelers-${copy}-${++mounted}`;
  let fonts = {};
  let adapts;
  let drawnFor = -1;
  let stale = false;
  let waiting = false;
  let destroyed = false;
  const fit = (style, fresh) => {
    const { onRender, onError, sanitize, ...rest } = settings;
    const base = {
      ...rest,
      idPrefix: rest.idPrefix ?? idPrefix,
      fontFamily: rest.fontFamily ?? fonts.family,
      fontFamilyMono: rest.fontFamilyMono ?? fonts.mono,
      maxWidth: undefined,
    };
    if (fresh) {
      natural = engine.render(model, base);
      adapts = undefined;
    }
    const available = settings.maxWidth ?? room(element, style);
    drawnFor = available;
    if (available > 0 && available < natural.width) {
      adapts ??=
        engine.render(model, { ...base, maxWidth: 1 }).svg !== natural.svg;
      if (adapts) return engine.render(model, { ...base, maxWidth: available });
    }
    return natural;
  };
  const put = (svg) => {
    if (settings.sanitize) element.replaceChildren(settings.sanitize(svg));
    else element.innerHTML = svg;
  };
  let errorElement;
  const mark = () => {
    const drawing = shown ? element.firstElementChild : null;
    if (drawing?.style)
      drawing.style.opacity = error === undefined ? "" : "0.4";
    if (error === undefined) {
      errorElement?.remove();
      errorElement = undefined;
      return;
    }
    const doc = element.ownerDocument;
    if (!doc) return;
    errorElement ??= doc.createElement("div");
    errorElement.className = "pelers pelers-error";
    const message = messageOf(error);
    if (drawing) {
      errorElement.textContent = summary(message);
      errorElement.title = message;
      errorElement.setAttribute("style", OVER_STYLE);
      if (errorElement.parentNode !== element) element.append(errorElement);
    } else {
      errorElement.textContent = message;
      errorElement.removeAttribute("title");
      errorElement.setAttribute("style", ALONE_STYLE);
      element.replaceChildren(errorElement);
    }
  };
  const fail = (failure) => {
    error = failure;
    mark();
    settings.onError?.(failure);
  };
  const awaitFonts = () => {
    if (waiting || destroyed) return;
    const loading = fontSets(element).filter(
      (fonts) => fonts.status === "loading",
    );
    if (loading.length === 0) return;
    waiting = true;
    afterFonts(loading, new WeakRef(handle));
  };
  const fontsLoaded = () => {
    waiting = false;
    stale = true;
    refresh();
  };
  const draw = (fresh, quiet = !fresh) => {
    const style = windowOf(element).getComputedStyle(element);
    const drawing = element.firstElementChild ?? null;
    const before = { fonts, natural, adapts, drawnFor, stale };
    let next;
    try {
      if (fresh) {
        fonts = fontsFor(style, drawing);
        stale = element.isConnected === false;
      }
      next = fit(style, fresh);
      if (quiet && next.svg === shown?.svg) {
        if (fresh) drawnModel = model;
        if (current && error !== undefined) {
          error = undefined;
          mark();
        }
        return;
      }
      put(next.svg);
    } catch (failure) {
      ({ fonts, natural, adapts, drawnFor, stale } = before);
      throw failure;
    }
    if (fresh) drawnModel = model;
    if (fresh && !drawing && element.firstElementChild) {
      const seen = fontsFor(style, element.firstElementChild);
      if (seen.family !== fonts.family || seen.mono !== fonts.mono) {
        const first = { fonts, natural, adapts, drawnFor };
        fonts = seen;
        try {
          const second = fit(style, true);
          put(second.svg);
          next = second;
        } catch {
          ({ fonts, natural, adapts, drawnFor } = first);
        }
      }
    }
    shown = next;
    if (current) error = undefined;
    mark();
    zoomer?.refresh();
    awaitFonts();
    settings.onRender?.(next);
  };
  let zoomer;
  let zooming = false;
  const zoom = () => {
    const wanted = settings.zoom ?? "auto";
    if (wanted === zooming) return;
    zoomer?.destroy();
    zooming = wanted;
    zoomer =
      wanted === false
        ? undefined
        : zoomWith(
            element,
            typeof wanted === "object" ? wanted : { always: wanted === true },
            false,
          );
  };
  const hidden = () =>
    element.isConnected === false || !(element.clientWidth > 0);
  const waits = () =>
    element.isConnected !== false && !(element.clientWidth > 0);
  const refresh = () => {
    if (destroyed) return;
    watch();
    if (!model || hidden()) return;
    const style = windowOf(element).getComputedStyle(element);
    const now = fontsFor(style, element.firstElementChild ?? null);
    try {
      if (stale || now.family !== fonts.family || now.mono !== fonts.mono) {
        draw(true, true);
      } else if (adapts !== false && settings.maxWidth === undefined) {
        const available = room(element, style);
        if (available !== drawnFor && available !== shown?.width) draw(false);
      }
    } catch (failure) {
      stale = false;
      if (current) error = failure;
      if (model !== drawnModel) {
        model = drawnModel;
        current = false;
      }
      mark();
      settings.onError?.(failure);
    }
    zoomer?.resized();
    awaitFonts();
  };
  let view;
  let observer;
  let frame;
  const cancelFrame = () => {
    if (frame !== undefined) view?.cancelAnimationFrame(frame);
    frame = undefined;
  };
  const watch = () => {
    const next = windowOf(element);
    if (next === view) return;
    observer?.disconnect();
    cancelFrame();
    view = next;
    observer = next.ResizeObserver
      ? new next.ResizeObserver(() => {
          if (frame !== undefined) return;
          if (!next.requestAnimationFrame) return refresh();
          frame = next.requestAnimationFrame(() => {
            frame = undefined;
            refresh();
          });
        })
      : undefined;
    observer?.observe(element);
  };
  const handle = {
    get result() {
      return shown;
    },
    get source() {
      return source;
    },
    get error() {
      return error;
    },
    get zoom() {
      return zoomer;
    },
    update(next, nextOptions = settings) {
      const reparse =
        !current ||
        next !== source ||
        nextOptions.limit !== settings.limit ||
        nextOptions.maxEdges !== settings.maxEdges ||
        nextOptions.config !== settings.config;
      source = next;
      settings = nextOptions;
      let nextModel = model;
      if (reparse) {
        try {
          nextModel = engine.parse(next, nextOptions);
        } catch (failure) {
          current = false;
          fail(failure);
          zoom();
          return;
        }
      }
      const was = shown;
      model = nextModel;
      current = true;
      if (waits()) {
        stale = true;
        zoom();
        return;
      }
      try {
        draw(true);
      } catch (failure) {
        if (shown !== was) {
          zoom();
          throw failure;
        }
        model = drawnModel;
        current = false;
        fail(failure);
      }
      zoom();
    },
    refresh,
    destroy() {
      destroyed = true;
      observer?.disconnect();
      observer = undefined;
      cancelFrame();
      view = undefined;
      zoomer?.destroy();
      onFontsLoaded.delete(handle);
      if (mounts.get(element) === handle) mounts.delete(element);
    },
  };
  onFontsLoaded.set(handle, fontsLoaded);
  try {
    model = engine.parse(text, options);
    current = true;
    draw(true);
  } catch (failure) {
    if (shown) throw failure;
    model = drawnModel;
    current = false;
    fail(failure);
  }
  zoom();
  watch();
  mounts.set(element, handle);
  return handle;
}
