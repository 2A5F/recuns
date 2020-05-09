#![allow(unused_variables, unused_mut, unused_imports, dead_code)]
use crate::*;

fn json() {}

mod token {
    use crate::*;
    use std::str::*;

    struct TokenData {}

    #[test]
    fn test_tokens() {
        let code = "asd";
        tokens(code.chars());
    }
    fn tokens(mut code: Chars) {
        let r = root.recuns();
        let r = do_loop((), r, true, || code.next().map(|v| Ok(v)));
        println!("{:?}", r);
    }

    fn root(inp: char, data: &mut ()) -> RecunsFlow<char, ()> {
        todo!()
    }
}
