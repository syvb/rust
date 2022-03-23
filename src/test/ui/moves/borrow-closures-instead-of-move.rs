fn takes_fn(f: impl Fn()) {
    loop {
        takes_fnonce(f);
        //~^ ERROR use of moved value
        //~| HELP consider borrowing
    }
}

fn takes_fn_mut(m: impl FnMut()) {
    if maybe() {
        takes_fnonce(m);
        //~^ HELP consider mutably borrowing
    }
    takes_fnonce(m);
    //~^ ERROR use of moved value
}

fn has_closure() {
    let mut x = 0;
    let closure = || {
        x += 1;
    };
    takes_fnonce(closure);
    closure();
}

fn maybe() -> bool {
    false
}

// Could also be Fn[Mut], here it doesn't matter
fn takes_fnonce(_: impl FnOnce()) {}

fn main() {}
