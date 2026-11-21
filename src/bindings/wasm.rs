use std::cell::RefCell;
use std::rc::Rc;

use js_sys::{Array, Function};
use wasm_bindgen::prelude::*;

use crate::api::{ParseOptions, ParsedDiagram, parse_with, render_with};
use crate::diagrams::gantt::jsdate::{OffsetFn, with_offsets};
use crate::svg::root::RenderOptions;
use crate::text::icons::IconFn;
use crate::text::math::{MathFn, MathMarkup};
use crate::text::measurer::MeasureFn;
use crate::util::hook::Hook;
use crate::util::json;
use crate::util::value::Value;
use crate::{PelersError, PelersErrorCode, RenderResult, detect_diagram, preprocess};

type Failure = Rc<RefCell<Option<JsValue>>>;

#[wasm_bindgen]
pub struct Parsed {
    inner: ParsedDiagram,
}

#[wasm_bindgen]
impl Parsed {
    #[wasm_bindgen(getter, js_name = "type")]
    pub fn diagram_type(&self) -> String {
        self.inner.diagram_type.as_str().to_owned()
    }

    #[wasm_bindgen(getter, js_name = "mermaidType")]
    pub fn mermaid_type(&self) -> String {
        self.inner.mermaid_type.as_str().to_owned()
    }

    #[wasm_bindgen(getter)]
    pub fn title(&self) -> Option<String> {
        self.inner.title.clone()
    }
}

#[wasm_bindgen(inline_js = "
export function rendered(svg, width, height, type, mermaidType, links, title) {
  const result = { svg, width, height, links: links ?? [], type, mermaidType };
  if (title !== undefined) result.title = title;
  return result;
}
export function link(id, href, internal) {
  return { id, href, internal };
}
export function failure(message, code, type, mermaidType, line, column, snippet) {
  return { message, code, type: type ?? null, mermaidType: mermaidType ?? null, line, column, snippet };
}
const utf8 = new TextDecoder();
let memory;
let bytes;
export function attach(value) {
  memory = value;
}
function text(at, length) {
  if (bytes === undefined || bytes.buffer !== memory.buffer) bytes = new Uint8Array(memory.buffer);
  if (length < 64) {
    let out = '';
    for (let i = at, end = at + length; i < end; i++) {
      const byte = bytes[i];
      if (byte > 127) return utf8.decode(bytes.subarray(at, end));
      out += String.fromCharCode(byte);
    }
    return out;
  }
  return utf8.decode(bytes.subarray(at, at + length));
}
let thrown;
export function failed() {
  const out = thrown;
  thrown = undefined;
  return out;
}
export function measure(f, at, length, size, style) {
  try {
    const width = f(text(at, length), size, style);
    return typeof width === 'number' ? width : NaN;
  } catch (error) {
    thrown ??= [error];
    return NaN;
  }
}
export function offset(f, ms) {
  try {
    const value = f(ms);
    return typeof value === 'number' ? value : 0;
  } catch (error) {
    thrown ??= [error];
    return 0;
  }
}
export function icon(f, at, length) {
  try {
    const markup = f(text(at, length));
    return typeof markup === 'string' ? markup : undefined;
  } catch (error) {
    thrown ??= [error];
    return undefined;
  }
}
export function math(value) {
  return value === null || typeof value !== 'object'
    ? undefined
    : [value.svg, value.html, value.width, value.height];
}
")]
extern "C" {
    fn rendered(
        svg: &str,
        width: f64,
        height: f64,
        kind: &JsValue,
        mermaid: &JsValue,
        links: JsValue,
        title: Option<String>,
    ) -> JsValue;
    fn link(id: &str, href: &str, internal: bool) -> JsValue;
    #[allow(clippy::too_many_arguments)]
    fn failure(
        message: &str,
        code: &JsValue,
        kind: Option<String>,
        mermaid: Option<String>,
        line: f64,
        column: f64,
        snippet: &str,
    ) -> JsValue;
    fn math(value: &JsValue) -> Option<Array>;
    fn attach(memory: JsValue);
    fn failed() -> Option<Array>;
    fn measure(f: &Function, at: u32, length: u32, size: f64, style: u32) -> f64;
    fn icon(f: &Function, at: u32, length: u32) -> Option<String>;
    fn offset(f: &Function, ms: f64) -> f64;
}

fn address(text: &str) -> (u32, u32) {
    (text.as_ptr() as u32, text.len() as u32)
}

#[wasm_bindgen]
extern "C" {
    pub type Options;
    #[wasm_bindgen(method, getter)]
    fn limit(this: &Options) -> Option<f64>;
    #[wasm_bindgen(method, getter, js_name = maxEdges)]
    fn max_edges(this: &Options) -> Option<f64>;
    #[wasm_bindgen(method, getter)]
    fn config(this: &Options) -> Option<String>;
    #[wasm_bindgen(method, getter, js_name = fontFamily)]
    fn font_family(this: &Options) -> Option<String>;
    #[wasm_bindgen(method, getter, js_name = fontFamilyMono)]
    fn font_family_mono(this: &Options) -> Option<String>;
    #[wasm_bindgen(method, getter)]
    fn monospace(this: &Options) -> Option<Vec<String>>;
    #[wasm_bindgen(method, getter, js_name = fontSize)]
    fn font_size(this: &Options) -> Option<f64>;
    #[wasm_bindgen(method, getter, js_name = idPrefix)]
    fn id_prefix(this: &Options) -> Option<String>;
    #[wasm_bindgen(method, getter)]
    fn responsive(this: &Options) -> Option<bool>;
    #[wasm_bindgen(method, getter, js_name = maxWidth)]
    fn max_width(this: &Options) -> Option<f64>;
    #[wasm_bindgen(method, getter, js_name = autoDirection)]
    fn auto_direction(this: &Options) -> Option<bool>;
    #[wasm_bindgen(method, getter, js_name = directionBreakpoint)]
    fn direction_breakpoint(this: &Options) -> Option<f64>;
    #[wasm_bindgen(method, getter)]
    fn padding(this: &Options) -> Option<f64>;
    #[wasm_bindgen(method, getter, js_name = outputLimit)]
    fn output_limit(this: &Options) -> Option<f64>;
    #[wasm_bindgen(method, getter, js_name = linkSchemes)]
    fn link_schemes(this: &Options) -> Option<Vec<String>>;
    #[wasm_bindgen(method, getter, js_name = imageSchemes)]
    fn image_schemes(this: &Options) -> Option<Vec<String>>;
    #[wasm_bindgen(method, getter)]
    fn links(this: &Options) -> Option<bool>;
    #[wasm_bindgen(method, getter)]
    fn images(this: &Options) -> Option<bool>;
    #[wasm_bindgen(method, getter, js_name = linkRel)]
    fn link_rel(this: &Options) -> Option<String>;
}

thread_local! {
    static NAMES: RefCell<Vec<(&'static str, JsValue)>> = const { RefCell::new(Vec::new()) };
    static HOST: RefCell<Option<Rc<OffsetFn>>> = const { RefCell::new(None) };
}

fn name(text: &'static str) -> JsValue {
    NAMES.with(|names| {
        let mut names = names.borrow_mut();
        if let Some((_, value)) = names.iter().find(|(known, _)| *known == text) {
            return value.clone();
        }
        let value = JsValue::from_str(text);
        names.push((text, value.clone()));
        value
    })
}

fn count(value: f64) -> usize {
    if value.is_nan() {
        usize::MAX
    } else {
        value as usize
    }
}

fn code(error: &PelersError) -> &'static str {
    match error.code {
        PelersErrorCode::UnsupportedDiagram => "unsupported-diagram",
        PelersErrorCode::Syntax => "syntax",
        PelersErrorCode::Semantic => "semantic",
        PelersErrorCode::Limit => "limit",
    }
}

fn error_object(error: PelersError) -> JsValue {
    failure(
        &error.message,
        &name(code(&error)),
        error.diagram_type,
        error.mermaid_type,
        error.line as f64,
        error.column as f64,
        &error.snippet,
    )
}

fn result_object(rendered_result: RenderResult, model: &ParsedDiagram) -> JsValue {
    let links = if rendered_result.links.is_empty() {
        JsValue::UNDEFINED
    } else {
        let links = Array::new();
        for entry in &rendered_result.links {
            links.push(&link(&entry.id, &entry.href, entry.internal));
        }
        links.into()
    };
    rendered(
        &rendered_result.svg,
        rendered_result.width,
        rendered_result.height,
        &name(model.diagram_type.as_str()),
        &name(model.mermaid_type.as_str()),
        links,
        rendered_result.title,
    )
}

fn outcome(rendered: Result<RenderResult, PelersError>, model: Option<&ParsedDiagram>) -> JsValue {
    match (rendered, model) {
        (Ok(rendered), Some(model)) => result_object(rendered, model),
        (Err(error), _) => error_object(error),
        (Ok(_), None) => unreachable!("a rendered diagram has a model"),
    }
}

fn parse_options(options: Option<&Options>) -> ParseOptions {
    let Some(options) = options else {
        return ParseOptions::default();
    };
    ParseOptions {
        limit: options.limit().map(count),
        max_edges: options.max_edges().map(count),
        config: options.config().and_then(|text| match json::parse(&text) {
            Some(Value::Object(config)) => Some(config),
            _ => None,
        }),
    }
}

fn calling(function: Option<Function>, failed: &Failure) -> Option<(Function, Failure)> {
    function.map(|function| (function, Rc::clone(failed)))
}

fn failing<T>(failed: &Failure, error: JsValue, fallback: T) -> T {
    failed.borrow_mut().get_or_insert(error);
    fallback
}

fn render_options(
    now: f64,
    options: Option<&Options>,
    icons: Option<Function>,
    math_hook: Option<Function>,
    measurer: Option<Function>,
    failed: &Failure,
) -> RenderOptions {
    let mut out = match options {
        Some(options) => RenderOptions {
            font_size: options.font_size(),
            font_family: options.font_family(),
            font_family_mono: options.font_family_mono(),
            padding: options.padding(),
            max_width: options.max_width(),
            auto_direction: options.auto_direction(),
            direction_breakpoint: options.direction_breakpoint(),
            responsive: options.responsive(),
            id_prefix: options.id_prefix(),
            monospace: options.monospace(),
            output_limit: options.output_limit().map(count),
            link_schemes: options.link_schemes(),
            image_schemes: options.image_schemes(),
            links: options.links(),
            images: options.images(),
            link_rel: options.link_rel(),
            ..RenderOptions::default()
        },
        None => RenderOptions::default(),
    };
    out.now = Some(now);
    if let Some(function) = icons {
        let resolve = move |name: &str| {
            let (at, length) = address(name);
            icon(&function, at, length)
        };
        out.icons = Some(Hook(Rc::new(resolve) as Rc<IconFn>));
    }
    if let Some((function, failed)) = calling(math_hook, failed) {
        let resolve = move |tex: &str, size: f64| match function.call2(
            &JsValue::NULL,
            &JsValue::from_str(tex),
            &JsValue::from_f64(size),
        ) {
            Ok(value) => math(&value).and_then(|parts| {
                let width = parts.get(2).as_f64().unwrap_or(f64::NAN);
                let height = parts.get(3).as_f64().unwrap_or(f64::NAN);
                match (parts.get(0).as_string(), parts.get(1).as_string()) {
                    (Some(svg), _) => Some(MathMarkup::Svg { svg, width, height }),
                    (None, Some(html)) => Some(MathMarkup::Html {
                        html,
                        width,
                        height,
                    }),
                    (None, None) => None,
                }
            }),
            Err(error) => failing(&failed, error, None),
        };
        out.math = Some(Hook(Rc::new(resolve) as Rc<MathFn>));
    }
    if let Some(function) = measurer {
        let measured = move |text: &str, size: f64, style: u32| {
            let (at, length) = address(text);
            measure(&function, at, length, size, style)
        };
        out.measurer = Some(Hook(Rc::new(measured) as Rc<MeasureFn>));
    }
    out
}

#[wasm_bindgen(js_name = "setOffsets")]
pub fn set_offsets(offsets: Function) {
    attach(wasm_bindgen::memory());
    let host = move |ms: f64| offset(&offsets, ms);
    HOST.with(|slot| *slot.borrow_mut() = Some(Rc::new(host)));
}

fn offsets_hook() -> Option<Hook<OffsetFn>> {
    HOST.with(|slot| slot.borrow().clone().map(Hook))
}

fn finish(failed: &Failure, value: JsValue) -> Result<JsValue, JsValue> {
    let thrown = self::failed().map(|error| error.get(0));
    match failed.borrow_mut().take().or(thrown) {
        Some(error) => Err(error),
        None => Ok(value),
    }
}

#[wasm_bindgen]
pub fn parse(source: &str, options: Option<Options>) -> Result<JsValue, JsValue> {
    let failed = Failure::default();
    let parsed = with_offsets(offsets_hook(), || {
        parse_with(source, &parse_options(options.as_ref()))
    });
    let value = match parsed {
        Ok(inner) => JsValue::from(Parsed { inner }),
        Err(error) => error_object(error),
    };
    finish(&failed, value)
}

#[wasm_bindgen(js_name = "renderSource")]
pub fn render_source(
    source: &str,
    now: f64,
    options: Option<Options>,
    icons: Option<Function>,
    math_hook: Option<Function>,
    measurer: Option<Function>,
) -> Result<JsValue, JsValue> {
    let failed = Failure::default();
    let parse = parse_options(options.as_ref());
    let options = render_options(now, options.as_ref(), icons, math_hook, measurer, &failed);
    let mut model = None;
    let rendered = with_offsets(offsets_hook(), || {
        let parsed = parse_with(source, &parse)?;
        let rendered = render_with(&parsed, &options);
        model = Some(parsed);
        rendered
    });
    finish(&failed, outcome(rendered, model.as_ref()))
}

#[wasm_bindgen(js_name = "renderParsed")]
pub fn render_parsed(
    model: &Parsed,
    now: f64,
    options: Option<Options>,
    icons: Option<Function>,
    math_hook: Option<Function>,
    measurer: Option<Function>,
) -> Result<JsValue, JsValue> {
    let failed = Failure::default();
    let options = render_options(now, options.as_ref(), icons, math_hook, measurer, &failed);
    let rendered = with_offsets(offsets_hook(), || render_with(&model.inner, &options));
    finish(&failed, outcome(rendered, Some(&model.inner)))
}

#[wasm_bindgen(js_name = "detectType")]
pub fn detect_type(source: &str) -> JsValue {
    match preprocess(source) {
        Ok(prepared) => detect_diagram(&prepared.text)
            .map_or(JsValue::NULL, |found| name(found.diagram_type.as_str())),
        Err(error) => error_object(error),
    }
}
