#![allow(unused_variables, unused_mut, unused_imports, dead_code)]
use crate::*;

fn json() {}

mod token {
    #![allow(non_upper_case_globals)]
    use crate::*;
    use batch_oper::*;
    use lazy_static::*;
    use regex::*;
    use std::collections::VecDeque;
    use std::iter::FromIterator;
    use std::str::*;

    #[derive(Debug, Clone, PartialEq)]
    enum Token {
        Str(String),
        Num(f64),
        Bool(bool),
        Null,
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
            |d| {
                code.next().map(|v| {
                    d.index += 1;
                    Ok(v)
                })
            },
            |d| {
                if d.tokens.is_empty() {
                    return None;
                }
                let y = VecDeque::from_iter(d.tokens.iter().cloned());
                d.tokens = vec![];
                Some(y)
            },
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
        }
        None
    }
    fn check_string(first: char, sp: usize) -> Option<Flow> {
        if first == '"' {
            let mut strs = vec![];
            return move |inp, data: &mut TokenData, eof| -> Flow {
                if eof || inp == '\0' {
                    //todo error
                }
                //                  \b   \f
                if bop!(|| inp; ==; '', '', '\n', '\r', '\t') {
                    //todo error
                }
                if inp == '"' {
                    //todo end
                }
                try_ret!(check_escape(inp, data.save(), {
                    let strs: *mut Vec<char> = &mut strs;
                    move |c| unsafe { &mut *strs }.push(c)
                }));
                strs.push(inp);
                RecunsFlow::None
            }
            .rfcall_next()
            .into();
        }
        None
    }
    fn check_escape(first: char, sp: usize, mut cb: impl 'static + FnMut(char)) -> Option<Flow> {
        if first == '\\' {
            return move |inp, data: &mut TokenData, eof| -> Flow {
                if bop!(|| inp; ==; '\\', '"', '/', 'b', 'f', 'n', 'r', 't') {
                    cb(inp);
                    return RecunsFlow::End;
                } else if inp == 'u' {
                    let mut uc = vec![];
                    return move |inp: char, data: &mut TokenData, eof| -> Flow {
                        if !inp.is_ascii_hexdigit() {
                            //todo error
                        }
                        uc.push(inp);
                        todo!()
                    }
                    .rfcall_next();
                } else {
                    //todo error
                }
                RecunsFlow::None
            }
            .rfcall_next()
            .into();
        }
        None
    }
    fn check_word(first: char, sp: usize) -> Option<Flow> {
        todo!()
    }
    fn check_symbol(first: char, sp: usize) -> Option<Flow> {
        todo!()
    }
}
