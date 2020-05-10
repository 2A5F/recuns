// #![allow(unused_variables, unused_mut, unused_imports, dead_code)]
use crate::*;

fn json() {}

mod token {
    #![allow(non_upper_case_globals)]
    use crate::*;
    use anyhow::Error;
    use batch_oper::*;
    use lazy_static::*;
    use regex::*;
    use std::collections::VecDeque;
    use std::iter::FromIterator;
    use std::ops::*;
    use std::str::*;
    use thiserror::*;

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

    #[derive(Error, Debug)]
    enum TokenError {
        #[error("Token is not a legal number at {}..{}", .0.start, .0.end)]
        NotNum(Range<usize>),
        #[error("Need '{}' but find EOF at {}..{}", .0, .1, .1)]
        NeedButEof(char, usize),
        #[error("Need <{}> but find EOF at {}..{}", .0, .1, .1)]
        NeedSomeButEof(String, usize),
        #[error("Unexpected EOF at {}..{}", .0, .0)]
        Eof(usize),
        #[error("Special characters need to be escaped at {}..{}", .0, .0)]
        NeedEscape(usize),
        #[error("Illegal Escape symbol {} at {}..{}", .0, .1, .1)]
        IllegalEscape(char, usize),
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
        let code = "\"asd\\";
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
        println!("{:?}", errors);
    }

    fn root(inp: char, data: &mut TokenData, eof: bool) -> Flow {
        if eof || inp == '\0' {
            return RecunsFlow::End;
        }
        let sp = data.save();
        try_ret!(check_number(inp, sp));
        try_ret!(check_string(inp, sp));
        //todo error
        RecunsFlow::None
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
                    if !number_regex.is_match(&*s) {
                        let np = data.save();
                        return Error::new(TokenError::NotNum(sp..np)).into();
                    }
                    let f = s.parse::<f64>();
                    match f {
                        Ok(f) => {
                            data.tokens.push(Token::Num(f));
                            return RecunsFlow::EndReDo;
                        }
                        Err(_) => {
                            let np = data.save();
                            return Error::new(TokenError::NotNum(sp..np)).into();
                        }
                    }
                } else {
                    strs.push(inp);
                    return RecunsFlow::None;
                }
            }
            .rfcall_next()
            .into();
        }
        None
    }
    fn check_string(first: char, _: usize) -> Option<Flow> {
        if first == '"' {
            let mut strs = vec![];
            return move |inp, data: &mut TokenData, eof| -> Flow {
                if eof || inp == '\0' {
                    let np = data.save();
                    return Error::new(TokenError::NeedButEof('"', np)).into();
                }
                //                  \b   \f
                if bop!(|| inp; ==; '', '', '\n', '\r', '\t') {
                    let np = data.save();
                    return Error::new(TokenError::NeedEscape(np)).into();
                }
                if inp == '"' {
                    let s: String = strs.iter().collect();
                    data.tokens.push(Token::Str(s));
                    return RecunsFlow::End;
                }
                try_ret!(check_escape(inp, data.save(), &mut strs));
                strs.push(inp);
                RecunsFlow::None
            }
            .rfcall_next()
            .into();
        }
        None
    }
    fn check_escape(first: char, _: usize, strs: *mut Vec<char>) -> Option<Flow> {
        fn doesc(c: char) -> char {
            match c {
                '\\' | '"' | '/' => c,
                'b' => '',
                'f' => '',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                _ => panic!("never"),
            }
        }
        if first == '\\' {
            return move |inp, data: &mut TokenData, eof| -> Flow {
                if eof || inp == '\0' {
                    let np = data.save();
                    return Error::new(TokenError::NeedSomeButEof("Escape Character".into(), np))
                        .into();
                }
                if bop!(|| inp; ==; '\\', '"', '/', 'b', 'f', 'n', 'r', 't') {
                    unsafe { &mut *strs }.push(doesc(inp));
                    return RecunsFlow::End;
                } else if inp == 'u' {
                    let mut uc = vec![];
                    return move |inp: char, data: &mut TokenData, eof| -> Flow {
                        if eof || inp == '\0' {
                            let np = data.save();
                            return Error::new(TokenError::Eof(np)).into();
                        }
                        if !inp.is_ascii_hexdigit() {
                            let np = data.save();
                            return Error::new(TokenError::IllegalEscape(inp, np)).into();
                        }
                        uc.push(inp);
                        if uc.len() == 4 {
                            let s: String = uc.iter().collect();
                            let hex: u32 = u32::from_str_radix(&*s, 16).unwrap();
                            let c = std::char::from_u32(hex).unwrap();
                            unsafe { &mut *strs }.push(c);
                            return RecunsFlow::End;
                        }
                        RecunsFlow::None
                    }
                    .rfmov_next();
                } else {
                    let np = data.save();
                    return Error::new(TokenError::IllegalEscape(inp, np)).into();
                }
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
