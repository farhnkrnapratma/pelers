use std::cell::RefCell;
use std::rc::Rc;

use crate::api::{ParseOptions, ParsedDiagram, parse_with, render_with};
use crate::diagrams::gantt::jsdate::{OffsetFn, with_offsets};
use crate::svg::root::RenderOptions;
use crate::text::icons::IconFn;
use crate::text::math::{MathFn, MathMarkup};
use crate::text::measurer::MeasureFn;
use crate::util::hook::Hook;
use crate::util::json;
use crate::util::value::Value;
use crate::{PelersError, PelersErrorCode, detect_diagram, preprocess};
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[napi(object)]
pub struct NativeLink {
    pub id: String,
    pub href: String,
    pub internal: bool,
}

#[napi(object)]
pub struct NativeResult {
    pub svg: String,
    pub width: f64,
    pub height: f64,
    pub links: Vec<NativeLink>,
    #[napi(js_name = "type")]
    pub diagram_type: String,
    pub mermaid_type: String,
    pub title: Option<String>,
}

#[napi(object)]
pub struct NativeError {
    pub message: String,
    pub code: String,
    #[napi(js_name = "type")]
    pub diagram_type: Option<String>,
    pub mermaid_type: Option<String>,
    pub line: u32,
    pub column: u32,
    pub snippet: String,
}

#[napi(object)]
pub struct NativeOutcome {
    pub result: Option<NativeResult>,
    pub error: Option<NativeError>,
}

#[napi(object)]
pub struct NativeParseOptions {
    pub limit: Option<f64>,
    pub max_edges: Option<f64>,
    pub config: Option<String>,
}

#[napi(object)]
pub struct NativeRenderOptions {
    pub font_family: Option<String>,
    pub font_family_mono: Option<String>,
    pub monospace: Option<Vec<String>>,
    pub font_size: Option<f64>,
    pub id_prefix: Option<String>,
    pub responsive: Option<bool>,
    pub max_width: Option<f64>,
    pub auto_direction: Option<bool>,
    pub direction_breakpoint: Option<f64>,
    pub padding: Option<f64>,
    pub output_limit: Option<f64>,
    pub link_schemes: Option<Vec<String>>,
    pub image_schemes: Option<Vec<String>>,
    pub links: Option<bool>,
    pub images: Option<bool>,
    pub link_rel: Option<String>,
    pub now: Option<f64>,
}

#[napi(object)]
pub struct NativeMath {
    pub svg: Option<String>,
    pub html: Option<String>,
    pub width: f64,
    pub height: f64,
}

#[napi]
pub struct Parsed {
    inner: ParsedDiagram,
}

#[napi]
impl Parsed {
    #[napi(getter, js_name = "type")]
    pub fn diagram_type(&self) -> String {
        self.inner.diagram_type.as_str().to_owned()
    }

    #[napi(getter)]
    pub fn mermaid_type(&self) -> String {
        self.inner.mermaid_type.as_str().to_owned()
    }

    #[napi(getter)]
    pub fn title(&self) -> Option<String> {
        self.inner.title.clone()
    }
}

type Failure = Rc<RefCell<Option<Error>>>;
type Core<T> = std::result::Result<T, PelersError>;
type IconCallback<'a> = Function<'a, String, Option<String>>;
type MathCallback<'a> = Function<'a, FnArgs<(String, f64)>, Option<NativeMath>>;
type MeasureCallback<'a> = Function<'a, FnArgs<(String, f64, u32)>, f64>;
type OffsetCallback<'a> = Function<'a, f64, f64>;

fn code(error: &PelersError) -> &'static str {
    match error.code {
        PelersErrorCode::UnsupportedDiagram => "unsupported-diagram",
        PelersErrorCode::Syntax => "syntax",
        PelersErrorCode::Semantic => "semantic",
        PelersErrorCode::Limit => "limit",
    }
}

fn native_error(error: PelersError) -> NativeError {
    NativeError {
        code: code(&error).to_owned(),
        message: error.message,
        diagram_type: error.diagram_type,
        mermaid_type: error.mermaid_type,
        line: u32::try_from(error.line).unwrap_or(u32::MAX),
        column: u32::try_from(error.column).unwrap_or(u32::MAX),
        snippet: error.snippet,
    }
}

fn count(value: f64) -> usize {
    if value.is_nan() {
        usize::MAX
    } else {
        value as usize
    }
}

fn parse_options(options: Option<NativeParseOptions>) -> ParseOptions {
    let Some(options) = options else {
        return ParseOptions::default();
    };
    ParseOptions {
        limit: options.limit.map(count),
        max_edges: options.max_edges.map(count),
        config: options.config.and_then(|text| match json::parse(&text) {
            Some(Value::Object(config)) => Some(config),
            _ => None,
        }),
    }
}

fn catching_panics<T>(run: impl FnOnce() -> Core<T>) -> Core<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)).unwrap_or_else(|panic| {
        let message = panic
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| panic.downcast_ref::<&str>().map(|text| (*text).to_owned()))
            .unwrap_or_else(|| "unknown panic".to_owned());
        Err(PelersError::new(
            format!("render panicked: {message}"),
            PelersErrorCode::Semantic,
        ))
    })
}

struct Callbacks<'a> {
    icons: Option<IconCallback<'a>>,
    math: Option<MathCallback<'a>>,
    measurer: Option<MeasureCallback<'a>>,
}

fn render_options(
    env: Env,
    options: Option<NativeRenderOptions>,
    callbacks: Callbacks<'_>,
    failed: &Failure,
) -> Result<RenderOptions> {
    let mut out = match options {
        Some(options) => RenderOptions {
            font_size: options.font_size,
            font_family: options.font_family,
            font_family_mono: options.font_family_mono,
            padding: options.padding,
            max_width: options.max_width,
            auto_direction: options.auto_direction,
            direction_breakpoint: options.direction_breakpoint,
            responsive: options.responsive,
            id_prefix: options.id_prefix,
            monospace: options.monospace,
            now: options.now,
            output_limit: options.output_limit.map(count),
            link_schemes: options.link_schemes,
            image_schemes: options.image_schemes,
            links: options.links,
            images: options.images,
            link_rel: options.link_rel,
            ..RenderOptions::default()
        },
        None => RenderOptions::default(),
    };
    if let Some(icons) = callbacks.icons {
        let reference = icons.create_ref()?;
        let failed = Rc::clone(failed);
        let resolve = move |name: &str| {
            let called = reference
                .borrow_back(&env)
                .and_then(|function| function.call(name.to_owned()));
            called.unwrap_or_else(|error| {
                failed.borrow_mut().get_or_insert(error);
                None
            })
        };
        out.icons = Some(Hook(Rc::new(resolve) as Rc<IconFn>));
    }
    if let Some(math) = callbacks.math {
        let reference = math.create_ref()?;
        let failed = Rc::clone(failed);
        let resolve = move |tex: &str, size: f64| {
            let called = reference
                .borrow_back(&env)
                .and_then(|function| function.call((tex.to_owned(), size).into()));
            match called {
                Ok(Some(NativeMath {
                    svg: Some(svg),
                    width,
                    height,
                    ..
                })) => Some(MathMarkup::Svg { svg, width, height }),
                Ok(Some(NativeMath {
                    html: Some(html),
                    width,
                    height,
                    ..
                })) => Some(MathMarkup::Html {
                    html,
                    width,
                    height,
                }),
                Ok(_) => None,
                Err(error) => {
                    failed.borrow_mut().get_or_insert(error);
                    None
                }
            }
        };
        out.math = Some(Hook(Rc::new(resolve) as Rc<MathFn>));
    }
    if let Some(measurer) = callbacks.measurer {
        let reference = measurer.create_ref()?;
        let failed = Rc::clone(failed);
        let measure = move |text: &str, size: f64, style: u32| {
            let called = reference
                .borrow_back(&env)
                .and_then(|function| function.call((text.to_owned(), size, style).into()));
            called.unwrap_or_else(|error| {
                failed.borrow_mut().get_or_insert(error);
                0.0
            })
        };
        out.measurer = Some(Hook(Rc::new(measure) as Rc<MeasureFn>));
    }
    Ok(out)
}

fn offsets_hook(
    env: Env,
    offsets: Option<OffsetCallback<'_>>,
    failed: &Failure,
) -> Result<Option<Hook<OffsetFn>>> {
    let Some(offsets) = offsets else {
        return Ok(None);
    };
    let reference = offsets.create_ref()?;
    let failed = Rc::clone(failed);
    let offset = move |ms: f64| {
        let called = reference
            .borrow_back(&env)
            .and_then(|function| function.call(ms));
        called.unwrap_or_else(|error| {
            failed.borrow_mut().get_or_insert(error);
            0.0
        })
    };
    Ok(Some(Hook(Rc::new(offset) as Rc<OffsetFn>)))
}

fn outcome(rendered: Core<crate::RenderResult>, model: Option<&ParsedDiagram>) -> NativeOutcome {
    match rendered {
        Ok(rendered) => {
            let model = model.expect("a rendered diagram has a model");
            NativeOutcome {
                result: Some(NativeResult {
                    svg: rendered.svg,
                    width: rendered.width,
                    height: rendered.height,
                    links: rendered
                        .links
                        .into_iter()
                        .map(|link| NativeLink {
                            id: link.id,
                            href: link.href,
                            internal: link.internal,
                        })
                        .collect(),
                    diagram_type: model.diagram_type.as_str().to_owned(),
                    mermaid_type: model.mermaid_type.as_str().to_owned(),
                    title: rendered.title,
                }),
                error: None,
            }
        }
        Err(error) => NativeOutcome {
            result: None,
            error: Some(native_error(error)),
        },
    }
}

fn finish(failed: &Failure, outcome: NativeOutcome) -> Result<NativeOutcome> {
    match failed.borrow_mut().take() {
        Some(error) => Err(error),
        None => Ok(outcome),
    }
}

#[napi]
pub fn parse(
    env: Env,
    source: String,
    options: Option<NativeParseOptions>,
    offsets: Option<OffsetCallback<'_>>,
) -> Result<Either<Parsed, NativeError>> {
    let failed = Failure::default();
    let offsets = offsets_hook(env, offsets, &failed)?;
    let parsed = with_offsets(offsets, || {
        catching_panics(|| parse_with(&source, &parse_options(options)))
    });
    if let Some(error) = failed.borrow_mut().take() {
        return Err(error);
    }
    Ok(match parsed {
        Ok(inner) => Either::A(Parsed { inner }),
        Err(error) => Either::B(native_error(error)),
    })
}

#[napi(js_name = "renderSource")]
#[allow(clippy::too_many_arguments)]
pub fn render_source(
    env: Env,
    source: String,
    parse: Option<NativeParseOptions>,
    options: Option<NativeRenderOptions>,
    icons: Option<IconCallback<'_>>,
    math: Option<MathCallback<'_>>,
    measurer: Option<MeasureCallback<'_>>,
    offsets: Option<OffsetCallback<'_>>,
) -> Result<NativeOutcome> {
    let failed = Failure::default();
    let options = render_options(
        env,
        options,
        Callbacks {
            icons,
            math,
            measurer,
        },
        &failed,
    )?;
    let offsets = offsets_hook(env, offsets, &failed)?;
    let mut model = None;
    let rendered = with_offsets(offsets, || {
        catching_panics(|| {
            let parsed = parse_with(&source, &parse_options(parse))?;
            let rendered = render_with(&parsed, &options);
            model = Some(parsed);
            rendered
        })
    });
    finish(&failed, outcome(rendered, model.as_ref()))
}

#[napi(js_name = "renderParsed")]
#[allow(clippy::too_many_arguments)]
pub fn render_parsed(
    env: Env,
    model: &Parsed,
    options: Option<NativeRenderOptions>,
    icons: Option<IconCallback<'_>>,
    math: Option<MathCallback<'_>>,
    measurer: Option<MeasureCallback<'_>>,
    offsets: Option<OffsetCallback<'_>>,
) -> Result<NativeOutcome> {
    let failed = Failure::default();
    let options = render_options(
        env,
        options,
        Callbacks {
            icons,
            math,
            measurer,
        },
        &failed,
    )?;
    let offsets = offsets_hook(env, offsets, &failed)?;
    let rendered = with_offsets(offsets, || {
        catching_panics(|| render_with(&model.inner, &options))
    });
    finish(&failed, outcome(rendered, Some(&model.inner)))
}

#[napi(js_name = "detectType")]
pub fn detect_type(source: String) -> Either<Option<String>, NativeError> {
    match preprocess(&source) {
        Ok(prepared) => Either::A(
            detect_diagram(&prepared.text).map(|found| found.diagram_type.as_str().to_owned()),
        ),
        Err(error) => Either::B(native_error(error)),
    }
}
