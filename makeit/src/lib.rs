pub use makeit_derive::Builder;

// NOTE: This type isn't really *needed*, but having a trait means that we're resilient to cases
// where the type being built already has a method called `builder`. This is exidingly rare, but
// it is a good idea to account for it.
pub trait Buildable {
    type Builder;
    #[must_use]
    fn builder() -> Self::Builder;
}
