# Compile-time checked Builder pattern `derive` macro with zero-memory overhead
>  *This is very much a work-in-progress. PRs welcome to bring this to production quality welcome.*

The [Builder Pattern](https://en.wikipedia.org/wiki/Builder_pattern) is a design pattern to allow the construction of complex types one field at a time by calling methods on a builder type. This crate provides a `derive` macro that allows you to annotate any `struct` to create a type-level state machine that requires all mandatory fields to be set once and only at compile-time (otherwise you won't be able to call `.build()` on it).

```rust
use makeit::{Buildable, Builder};

#[derive(Builder)]
struct Foo<'a, T: std::fmt::Debug> {
    bar: &'a T,
    #[default(42)]
    baz: i32,
}

// This is the expected use
let x = Foo::builder().set_bar(&()).set_baz(42).build();
// The following builds because of the `default` annotation on `baz`
let x = Foo::builder().set_bar(&()).build();
// The following won't build because `bar` hasn't been set
let x = Foo::builder().set_baz(0).build();
```

You can look at the `examples` directory for a showcase of the available features.

The created `Builder` type has zero-memory overhead because it uses a [`MaybeUninit` backed field](https://lucumr.pocoo.org/2022/1/30/unsafe-rust/) of the type to be built.

## Error messages
One of the main downsides of the typestate pattern are inscrutable error messages. However, the since this crate generates type-state parameters with readable names, rustc can produce readable error messages:
![image](https://user-images.githubusercontent.com/492903/154131011-1bb5d95b-b00d-4ce6-8683-d86a01edd58f.png)

