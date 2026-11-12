use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::util::hook::Hook;
use crate::util::jsnum::js_number_to_string;

#[derive(Debug, Clone, PartialEq)]
pub enum MathMarkup {
    Svg {
        svg: String,
        width: f64,
        height: f64,
    },
    Html {
        html: String,
        width: f64,
        height: f64,
    },
}

impl MathMarkup {
    pub fn width(&self) -> f64 {
        match self {
            Self::Svg { width, .. } | Self::Html { width, .. } => *width,
        }
    }

    pub fn height(&self) -> f64 {
        match self {
            Self::Svg { height, .. } | Self::Html { height, .. } => *height,
        }
    }
}

pub type MathFn = dyn Fn(&str, f64) -> Option<MathMarkup>;

struct Scope {
    resolver: Option<Rc<MathFn>>,
    found: HashMap<String, Option<MathMarkup>>,
}

thread_local! {
    static SCOPE: RefCell<Option<Scope>> = const { RefCell::new(None) };
}

struct Restore(Option<Scope>);

impl Drop for Restore {
    fn drop(&mut self) {
        SCOPE.with(|scope| *scope.borrow_mut() = self.0.take());
    }
}

pub fn with_math<T>(math: Option<Hook<MathFn>>, run: impl FnOnce() -> T) -> T {
    let outer = SCOPE.with(|scope| {
        scope.replace(Some(Scope {
            resolver: math.map(|hook| hook.0),
            found: HashMap::new(),
        }))
    });
    let _restore = Restore(outer);
    run()
}

pub fn without_math<T>(run: impl FnOnce() -> T) -> T {
    let resolver = SCOPE.with(|scope| {
        scope
            .borrow_mut()
            .as_mut()
            .and_then(|scope| scope.resolver.take())
    });
    let _restore = MathRestore(resolver);
    run()
}

struct MathRestore(Option<Rc<MathFn>>);

impl Drop for MathRestore {
    fn drop(&mut self) {
        let resolver = self.0.take();
        SCOPE.with(|scope| {
            if let Some(scope) = scope.borrow_mut().as_mut() {
                scope.resolver = resolver;
            }
        });
    }
}

pub fn math_active() -> bool {
    SCOPE.with(|scope| {
        scope
            .borrow()
            .as_ref()
            .is_some_and(|scope| scope.resolver.is_some())
    })
}

fn valid(size: f64) -> bool {
    size.is_finite() && size > 0.0
}

pub fn typeset(tex: &str, size: f64) -> Option<MathMarkup> {
    let key = format!("{}\n{tex}", js_number_to_string(size));
    let resolver = SCOPE.with(|scope| {
        let scope = scope.borrow();
        let scope = scope.as_ref()?;
        let resolver = scope.resolver.clone()?;
        Some((resolver, scope.found.get(&key).cloned()))
    });
    let (resolver, cached) = resolver?;
    if let Some(found) = cached {
        return found;
    }
    let markup =
        resolver(tex, size).filter(|markup| valid(markup.width()) && valid(markup.height()));
    SCOPE.with(|scope| {
        if let Some(scope) = scope.borrow_mut().as_mut() {
            scope.found.insert(key, markup.clone());
        }
    });
    markup
}
