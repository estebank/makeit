#![deny(unused_must_use, clippy::pedantic)]
use makeit::{Buildable, Builder};

#[derive(Builder)]
struct Bar {
    bar: u32,
    #[default(32)]
    baz: Option<i32>,
}

pub fn y() {
    let x = Bar::builder().set_bar(1).set_baz(Some(2)).build();
    println!("{:?} {:?}", x.bar, x.baz);
    let x = Bar::builder().set_bar(1).build();
    println!("{:?} {:?}", x.bar, x.baz);
}

fn main() {
    y();
}
