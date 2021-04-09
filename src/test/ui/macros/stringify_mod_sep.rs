// run-pass

mod x {
    #[allow(dead_code)]
    pub fn y() {}
}

fn main() {
    assert_eq!(stringify!(x::y()), "x::y()");
    assert_eq!(stringify!( x :: y ( ) ), "x::y()");
}
