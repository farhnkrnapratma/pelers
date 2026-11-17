use std::fmt;
use std::rc::Rc;

pub struct Hook<T: ?Sized>(pub Rc<T>);

impl<T: ?Sized> Clone for Hook<T> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<T: ?Sized> fmt::Debug for Hook<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Hook")
    }
}

impl<T: ?Sized> PartialEq for Hook<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
