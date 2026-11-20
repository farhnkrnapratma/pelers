use std::cell::RefCell;
use std::rc::Rc;

use js_sys::{Array, Function, Object, Reflect};
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

fn get(object: &JsValue, key: &str) -> Option<JsValue> {
    if !object.is_object() {
        return None;
    }
    Reflect::get(object, &JsValue::from_str(key))
        .ok()
        .filter(|value| !value.is_undefined() && !value.is_null())
}

fn number(object: &JsValue, key: &str) -> Option<f64> {
    get(object, key).and_then(|value| value.as_f64())
}

fn text(object: &JsValue, key: &str) -> Option<String> {
    get(object, key).and_then(|value| value.as_string())
}

fn flag(object: &JsValue, key: &str) -> Option<bool> {
    get(object, key).and_then(|value| value.as_bool())
}

fn texts(object: &JsValue, key: &str) -> Option<Vec<String>> {
    get(object, key).filter(Array::is_array).map(|value| {
        Array::from(&value)
            .iter()
            .filter_map(|item| item.as_string())
            .collect()
    })
}

fn count(value: f64) -> usize {
    if value.is_nan() {
        usize::MAX
    } else {
        value as usize
    }
}

fn set(object: &Object, key: &str, value: impl Into<JsValue>) {
    let _ = Reflect::set(object, &JsValue::from_str(key), &value.into());
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
    let object = Object::new();
    set(&object, "message", error.message.as_str());
    set(&object, "code", code(&error));
    set(
        &object,
        "type",
        error.diagram_type.map_or(JsValue::NULL, JsValue::from),
    );
    set(
        &object,
        "mermaidType",
        error.mermaid_type.map_or(JsValue::NULL, JsValue::from),
    );
    set(&object, "line", error.line as f64);
    set(&object, "column", error.column as f64);
    set(&object, "snippet", error.snippet.as_str());
    object.into()
}

fn result_object(rendered: RenderResult, model: &ParsedDiagram) -> JsValue {
    let object = Object::new();
    set(&object, "svg", rendered.svg.as_str());
    set(&object, "width", rendered.width);
    set(&object, "height", rendered.height);
    let links = Array::new();
    for link in rendered.links {
        let entry = Object::new();
        set(&entry, "id", link.id.as_str());
        set(&entry, "href", link.href.as_str());
        set(&entry, "internal", link.internal);
        links.push(&entry);
    }
    set(&object, "links", links);
    set(&object, "type", model.diagram_type.as_str());
    set(&object, "mermaidType", model.mermaid_type.as_str());
    if let Some(title) = rendered.title {
        set(&object, "title", title.as_str());
    }
    object.into()
}

fn outcome(rendered: Result<RenderResult, PelersError>, model: Option<&ParsedDiagram>) -> JsValue {
    let object = Object::new();
    match (rendered, model) {
        (Ok(rendered), Some(model)) => set(&object, "result", result_object(rendered, model)),
        (Err(error), _) => set(&object, "error", error_object(error)),
        (Ok(_), None) => unreachable!("a rendered diagram has a model"),
    }
    object.into()
}

fn parse_options(options: &JsValue) -> ParseOptions {
    ParseOptions {
        limit: number(options, "limit").map(count),
        max_edges: number(options, "maxEdges").map(count),
        config: text(options, "config").and_then(|text| match json::parse(&text) {
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
    options: &JsValue,
    icons: Option<Function>,
    math: Option<Function>,
    measurer: Option<Function>,
    failed: &Failure,
) -> RenderOptions {
    let mut out = RenderOptions {
        font_size: number(options, "fontSize"),
        font_family: text(options, "fontFamily"),
        font_family_mono: text(options, "fontFamilyMono"),
        padding: number(options, "padding"),
        max_width: number(options, "maxWidth"),
        auto_direction: flag(options, "autoDirection"),
        direction_breakpoint: number(options, "directionBreakpoint"),
        responsive: flag(options, "responsive"),
        id_prefix: text(options, "idPrefix"),
        monospace: texts(options, "monospace"),
        now: number(options, "now"),
        output_limit: number(options, "outputLimit").map(count),
        link_schemes: texts(options, "linkSchemes"),
        image_schemes: texts(options, "imageSchemes"),
        links: flag(options, "links"),
        images: flag(options, "images"),
        link_rel: text(options, "linkRel"),
        ..RenderOptions::default()
    };
    if let Some((function, failed)) = calling(icons, failed) {
        let resolve =
            move |name: &str| match function.call1(&JsValue::NULL, &JsValue::from_str(name)) {
                Ok(value) => value.as_string(),
                Err(error) => failing(&failed, error, None),
            };
        out.icons = Some(Hook(Rc::new(resolve) as Rc<IconFn>));
    }
    if let Some((function, failed)) = calling(math, failed) {
        let resolve = move |tex: &str, size: f64| match function.call2(
            &JsValue::NULL,
            &JsValue::from_str(tex),
            &JsValue::from_f64(size),
        ) {
            Ok(value) => {
                let width = number(&value, "width").unwrap_or(f64::NAN);
                let height = number(&value, "height").unwrap_or(f64::NAN);
                match (text(&value, "svg"), text(&value, "html")) {
                    (Some(svg), _) => Some(MathMarkup::Svg { svg, width, height }),
                    (None, Some(html)) => Some(MathMarkup::Html {
                        html,
                        width,
                        height,
                    }),
                    (None, None) => None,
                }
            }
            Err(error) => failing(&failed, error, None),
        };
        out.math = Some(Hook(Rc::new(resolve) as Rc<MathFn>));
    }
    if let Some((function, failed)) = calling(measurer, failed) {
        let measure = move |text: &str, size: f64, style: u32| match function.call3(
            &JsValue::NULL,
            &JsValue::from_str(text),
            &JsValue::from_f64(size),
            &JsValue::from_f64(f64::from(style)),
        ) {
            Ok(value) => value.as_f64().unwrap_or(f64::NAN),
            Err(error) => failing(&failed, error, f64::NAN),
        };
        out.measurer = Some(Hook(Rc::new(measure) as Rc<MeasureFn>));
    }
    out
}

fn offsets_hook(offsets: Option<Function>, failed: &Failure) -> Option<Hook<OffsetFn>> {
    calling(offsets, failed).map(|(function, failed)| {
        let offset = move |ms: f64| match function.call1(&JsValue::NULL, &JsValue::from_f64(ms)) {
            Ok(value) => value.as_f64().unwrap_or(0.0),
            Err(error) => failing(&failed, error, 0.0),
        };
        Hook(Rc::new(offset) as Rc<OffsetFn>)
    })
}

fn finish(failed: &Failure, value: JsValue) -> Result<JsValue, JsValue> {
    match failed.borrow_mut().take() {
        Some(error) => Err(error),
        None => Ok(value),
    }
}

#[wasm_bindgen]
pub fn parse(
    source: &str,
    options: JsValue,
    offsets: Option<Function>,
) -> Result<JsValue, JsValue> {
    let failed = Failure::default();
    let parsed = with_offsets(offsets_hook(offsets, &failed), || {
        parse_with(source, &parse_options(&options))
    });
    let value = match parsed {
        Ok(inner) => JsValue::from(Parsed { inner }),
        Err(error) => error_object(error),
    };
    finish(&failed, value)
}

#[wasm_bindgen(js_name = "renderSource")]
#[allow(clippy::too_many_arguments)]
pub fn render_source(
    source: &str,
    parse: JsValue,
    options: JsValue,
    icons: Option<Function>,
    math: Option<Function>,
    measurer: Option<Function>,
    offsets: Option<Function>,
) -> Result<JsValue, JsValue> {
    let failed = Failure::default();
    let options = render_options(&options, icons, math, measurer, &failed);
    let mut model = None;
    let rendered = with_offsets(offsets_hook(offsets, &failed), || {
        let parsed = parse_with(source, &parse_options(&parse))?;
        let rendered = render_with(&parsed, &options);
        model = Some(parsed);
        rendered
    });
    finish(&failed, outcome(rendered, model.as_ref()))
}

#[wasm_bindgen(js_name = "renderParsed")]
pub fn render_parsed(
    model: &Parsed,
    options: JsValue,
    icons: Option<Function>,
    math: Option<Function>,
    measurer: Option<Function>,
    offsets: Option<Function>,
) -> Result<JsValue, JsValue> {
    let failed = Failure::default();
    let options = render_options(&options, icons, math, measurer, &failed);
    let rendered = with_offsets(offsets_hook(offsets, &failed), || {
        render_with(&model.inner, &options)
    });
    finish(&failed, outcome(rendered, Some(&model.inner)))
}

#[wasm_bindgen(js_name = "detectType")]
pub fn detect_type(source: &str) -> JsValue {
    match preprocess(source) {
        Ok(prepared) => detect_diagram(&prepared.text).map_or(JsValue::NULL, |found| {
            JsValue::from_str(found.diagram_type.as_str())
        }),
        Err(error) => error_object(error),
    }
}
