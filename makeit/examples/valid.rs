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
    let x: Foo<_> = Foo::builder().set_bar(&()).build();
    println!("{:?} {:?}", x.bar, x.baz);
}

#[derive(Builder)]
struct Bar {
    bar: u32,
    #[default]
    baz: Option<i32>,
}

pub fn y() {
    let x: Bar = Bar::builder().set_bar(1).set_baz(Some(2)).build();
    println!("{:?} {:?}", x.bar, x.baz);
    let x = Bar::builder().set_bar(1).build();
    println!("{:?} {:?}", x.bar, x.baz);
}

#[derive(Builder)]
struct Baz<'a, T>(&'a T, #[default] Option<i32>)
where
    T: std::fmt::Debug;

pub fn z() {
    let x = Baz::builder().set_0(&1).set_1(Some(2)).build();
    println!("{:?} {:?}", x.0, x.1);
    let x = Baz::builder().set_0(&1).build();
    println!("{:?} {:?}", x.0, x.1);
}

fn main() {
    x();
    y();
}
