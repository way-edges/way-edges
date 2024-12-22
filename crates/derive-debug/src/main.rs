use way_edges_derive::{wrap_rc, GetSize};

#[wrap_rc(rc = "pub")]
#[derive(GetSize)]
struct Test;

fn main() {
    TestRc;
    println!("Hello, world!");
}
