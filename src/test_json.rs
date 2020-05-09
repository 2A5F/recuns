#![allow(unused_variables, unused_mut, unused_imports, dead_code)]
use crate::*;

fn json() {}

mod token {
    #![allow(non_upper_case_globals)]
    use crate::*;
    use batch_oper::*;
    use lazy_static::*;
    use regex::*;
    use std::str::*;

    struct TokenData {}
    type Flow = RecunsFlow<char, TokenData>;

    #[test]
    fn test_tokens() {
        let code = "123";
        tokens(code.chars());
    }
    fn tokens(mut code: Chars) {
        let mut errors = vec![];
        let r = root.recuns();
        let r = do_iter(
            TokenData {},
            r,
            true,
            &mut errors,
            || code.next().map(|v| Ok(v)),
            |d| Some(()),
        );
        let r = r.collect::<Vec<_>>();
        println!("{:?}", r);
    }

    fn root(inp: char, data: &mut TokenData, eof: bool) -> Flow {
        check_number(inp).unwrap_or(RecunsFlow::None)
    }

    lazy_static! {
        static ref num_start: Regex = Regex::new(r"[\-\d\.]").unwrap();
    }
    fn check_number(inp: char) -> Option<Flow> {
        if bop!(|| inp; ==; '-', '.', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9') {
            return |inp, data: &mut TokenData, eof| -> Flow { 
                    RecunsFlow::End 
                }
                .rfcall()
                .into();
        };
        None
    }
}
