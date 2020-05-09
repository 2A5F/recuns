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

    enum Token {
        Str(String),
        Num(f64),
        ArrS,
        ArrE,
        ObjS,
        ObjE,
        Comma,
        Colon,
    }

    struct TokenData {
        index: usize,
        tokens: Vec<Token>,
    }
    impl TokenData {
        #[inline]
        pub fn save(&self) -> usize {
            self.index
        }
    }
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
            TokenData {
                index: 0,
                tokens: vec![],
            },
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
        let sp = data.save();
        check_number(inp, sp).unwrap_or(RecunsFlow::None)
    }

    lazy_static! {
        static ref number_regex: Regex =
            Regex::new(r"-?(([1-9]\d*)|0)(\.\d+)?([eE][-+]?\d+)?").unwrap();
    }
    fn check_number(first: char, sp: usize) -> Option<Flow> {
        #[inline(always)]
        fn is_num_start(c: char) -> bool {
            bop!(|| c; ==; '-', '.', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9')
        }
        #[inline(always)]
        fn is_num(c: char) -> bool {
            is_num_start(c) || bop!(|| c; ==; 'e', 'E', '+')
        }
        if is_num_start(first) {
            let mut strs = vec![first];
            return move |inp, data: &mut TokenData, eof| -> Flow {
                if eof || inp == '\0' || !is_num(inp) {
                    let s = strs.iter().collect::<String>();
                    let f = s.parse::<f64>();
                    match f {
                        Ok(f) => {
                            data.tokens.push(Token::Num(f));
                        }
                        Err(e) => {
                            return RecunsFlow::Err(Arc::new(e));
                        }
                    }
                } else {
                    strs.push(inp);
                    return RecunsFlow::None;
                }
                RecunsFlow::End
            }
            .rfcall_next()
            .into();
        };
        None
    }
}
