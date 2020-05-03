#![allow(unused_variables, unused_mut, unused_imports, dead_code)]
use crate::*;

fn json() {}

mod token {
    use crate::*;
    use std::str::*;

    struct TokenData {}

    fn tokens(mut code: Box<Chars>) {
        let mut state = State::<char, _>::new(false, TokenData {});
        //state.call(input)
    }

    fn root() {}
}
