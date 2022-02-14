#![deny(unused_must_use, clippy::pedantic)]
use makeit::{Buildable, Builder};

#[derive(Builder)]
struct Foo<'a, T: std::fmt::Debug> {
    bar: &'a T,
    #[default(42)]
    baz: i32,
}

pub fn x() {
    let x: Foo<_> = Foo::builder().set_bar(&()).set_baz(2).build();
    println!("{:?} {:?}", x.bar, x.baz);
    let x = Foo::builder().set_bar(&1).build();
    println!("{:?} {:?}", x.bar, x.baz);
}

#[derive(Builder)]
struct Bar {
    bar: u32,
    baz: i32,
}

pub fn y() {
    let x: Bar = Bar::builder().set_bar(1).set_baz(2).build();
    println!("{:?} {:?}", x.bar, x.baz);
    let _x = Bar::builder();
    let _x = Bar::builder().set_bar(1);
}

fn main() {
    x();
    y();
}
