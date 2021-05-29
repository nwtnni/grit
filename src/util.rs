pub trait Tap: Sized {
    fn tap<F: FnOnce(Self) -> T, T>(self, apply: F) -> T {
        apply(self)
    }

    fn tap_mut<F: FnOnce(&mut Self)>(mut self, apply: F) -> Self {
        apply(&mut self);
        self
    }
}

impl<T: Sized> Tap for T {}
