#![deny(unused_must_use, clippy::pedantic)]
use makeit::{Buildable, Builder};

#[derive(Debug)]
struct S;

#[derive(Builder)]
struct Foo<'a, T: std::fmt::Debug> {
    bar: &'a T,
    #[default]
    baz: S,
}

pub fn x() {
    let x: Foo<_> = Foo::builder().set_bar(&()).set_baz(S).build();
    println!("{:?} {:?}", x.bar, x.baz);
    let x: Foo<_> = Foo::builder().set_bar(&()).build();
    println!("{:?} {:?}", x.bar, x.baz);
}

fn main() {
    x();
}
