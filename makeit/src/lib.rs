pub use makeit_derive::Builder;

pub trait Buildable {
    type Builder;
    #[must_use]
    fn builder() -> Self::Builder;
}
