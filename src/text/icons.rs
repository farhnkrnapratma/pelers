use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::util::hook::Hook;

pub type IconFn = dyn Fn(&str) -> Option<String>;

struct Scope {
    resolver: Option<Rc<IconFn>>,
    found: HashMap<String, String>,
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

pub fn with_icons<T>(icons: Option<Hook<IconFn>>, run: impl FnOnce() -> T) -> T {
    let outer = SCOPE.with(|scope| {
        scope.replace(Some(Scope {
            resolver: icons.map(|hook| hook.0),
            found: HashMap::new(),
        }))
    });
    let _restore = Restore(outer);
    run()
}

pub fn icon_markup(name: &str) -> String {
    let resolved = SCOPE.with(|scope| {
        let scope = scope.borrow();
        let scope = scope.as_ref()?;
        let resolver = scope.resolver.clone()?;
        Some((resolver, scope.found.get(name).cloned()))
    });
    let Some((resolver, cached)) = resolved else {
        return String::new();
    };
    if let Some(markup) = cached {
        return markup;
    }
    let markup = resolver(name).unwrap_or_default();
    SCOPE.with(|scope| {
        if let Some(scope) = scope.borrow_mut().as_mut() {
            scope.found.insert(name.to_owned(), markup.clone());
        }
    });
    markup
}

pub fn resolve_icon(name: &str) -> String {
    let resolver = SCOPE.with(|scope| {
        scope
            .borrow()
            .as_ref()
            .and_then(|scope| scope.resolver.clone())
    });
    resolver
        .and_then(|resolver| resolver(name))
        .unwrap_or_default()
}
