// #![allow(unused_variables, unused_mut, unused_imports, dead_code)]
use crate::*;
use anyhow::Error;
use batch_oper::*;
use lazy_static::*;
use std::collections::BTreeMap;
use std::ops::*;
use std::str::*;
use thiserror::*;
use token::*;

static CODE: &'static str =
    "{ \"a\": 1, \"b\": true, \"c\": [null, 1.5, false], \"d\": { \"v\": \"asd\" } }";

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<JsonValue>),
    Obj(BTreeMap<String, JsonValue>),
}

#[derive(Debug)]
struct ParserData {
    out: Option<JsonValue>,
}

type Flow = RecunsFlow<Token, ParserData>;

#[test]
fn test_json() {
    let r = json(CODE.chars());
    println!("{:?}", r);
}

fn json(code: impl Iterator<Item = char>) -> Result<Option<JsonValue>, Vec<Arc<Error>>> {
    let tokens = tokens(code)?;
    let mut tokens = tokens.iter();
    let r = root.recuns();
    let r = do_loop(ParserData { out: None }, r, false, |_| {
        tokens.next().map(|v| Ok(v.clone()))
    })?;
    Ok(r.unwrap().out)
}

fn root(inp: Token, data: &mut ParserData, eof: bool) -> Flow {
    if eof {
        return Flow::End;
    }
    if data.out.is_some() {
        //todo error
    }
    let data: *mut ParserData = data;
    check_value(&inp, Box::new(move |v| unsafe { &mut *data }.out = Some(v)))
}
fn check_value(inp: &Token, mut cb: Box<dyn FnMut(JsonValue)>) -> Flow {
    try_ret!(check_literal(inp, |v| cb(v)));
    if let Some(f) = check_arr(inp) {
        return f(Box::new(move |v| cb(v)));
    }
    if let Some(f) = check_obj(inp) {
        return f(Box::new(move |v| cb(v)));
    }
    todo!()
    //todo error
}
fn check_literal(inp: &Token, mut cb: impl FnMut(JsonValue)) -> Option<Flow> {
    match inp {
        Token::Str(s) => cb(JsonValue::Str(s.clone())),
        &Token::Num(n) => cb(JsonValue::Num(n)),
        &Token::Bool(b) => cb(JsonValue::Bool(b)),
        Token::Null => cb(JsonValue::Null),
        _ => return None,
    }
    Some(Flow::End)
}
#[inline]
fn check_arr(inp: &Token) -> Option<impl FnOnce(Box<dyn FnMut(JsonValue)>) -> Flow> {
    if let Token::ArrS = inp {
        return Some(|mut cb: Box<dyn FnMut(JsonValue)>| {
            let mut vals = vec![];
            let mut split = false;
            return move |inp: Token, _: &mut ParserData, eof: bool| -> Flow {
                if eof {
                    //todo error
                }
                if let Token::ArrE = inp {
                    cb(JsonValue::Arr(std::mem::replace(&mut vals, vec![])));
                    return Flow::End;
                }
                if split {
                    split = false;
                    if let Token::Comma = inp {
                        return Flow::None;
                    } else {
                        //todo error
                    }
                } else {
                    let vals: *mut Vec<JsonValue> = &mut vals;
                    let split: *mut bool = &mut split;
                    check_value(
                        &inp,
                        Box::new(move |v| unsafe {
                            (&mut *vals).push(v);
                            *split = true;
                        }),
                    );
                }

                Flow::None
            }
            .rfcall_next();
        });
    }
    None
}
#[inline]
fn check_obj(inp: &Token) -> Option<impl FnOnce(Box<dyn FnMut(JsonValue)>) -> Flow> {
    if let Token::ObjS = inp {
        return Some(|mut cb: Box<dyn FnMut(JsonValue)>| {
            enum Need {
                Key,
                Colon,
                Value,
                Comma,
            }
            let mut vals = BTreeMap::new();
            let mut key = None;
            let mut need = Need::Key;
            return move |inp: Token, _: &mut ParserData, eof: bool| -> Flow {
                if eof {
                    //todo error
                }
                if let Token::ObjE = inp {
                    cb(JsonValue::Obj(std::mem::replace(
                        &mut vals,
                        BTreeMap::new(),
                    )));
                    return Flow::End;
                }
                match need {
                    Need::Key => {
                        if let Token::Str(k) = inp {
                            key = Some(k);
                            need = Need::Colon;
                        } else {
                            //todo error
                        }
                    }
                    Need::Colon => {
                        if let Token::Colon = inp {
                            need = Need::Value;
                        } else {
                            //todo error
                        }
                    }
                    Need::Value => {
                        let vals: *mut _ = &mut vals;
                        let key: *mut _ = &mut key;
                        let need: *mut _ = &mut need;
                        check_value(
                            &inp,
                            Box::new(move |v| unsafe {
                                let k = std::mem::replace(&mut *key, None).unwrap();
                                (&mut *vals).insert(k, v);
                                *need = Need::Comma;
                            }),
                        );
                    }
                    Need::Comma => {
                        if let Token::Comma = inp {
                            need = Need::Key;
                        } else {
                            //todo error
                        }
                    }
                }
                Flow::None
            }
            .rfcall_next();
        });
    }
    None
}

mod token {
    #![allow(non_upper_case_globals)]
    use super::*;
    use anyhow::Error;
    use regex::*;
    use std::collections::VecDeque;
    use std::iter::FromIterator;

    #[derive(Debug, Clone, PartialEq)]
    pub enum Token {
        None,
        Str(String),
        Num(f64),
        Bool(bool),
        Null,
        /// `[`
        ArrS,
        /// `]`
        ArrE,
        /// `{`
        ObjS,
        /// `}`
        ObjE,
        /// `,`
        Comma,
        /// `:`
        Colon,
    }
    impl Default for Token {
        fn default() -> Self {
            Self::None
        }
    }

    #[derive(Error, Debug, PartialEq, Eq)]
    pub enum TokenError {
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
        #[error("Illegal Escape symbol '{}' at {}..{}", .0, .1, .1)]
        IllegalEscape(char, usize),
        #[error("Unknown word \"{}\" at {}..{}", .0, .1.start, .1.end)]
        UnknownWord(String, Range<usize>),
        #[error( "Unknown character '{}' at {}..{}", .0, .1, .1)]
        UnknownCharacter(char, usize),
    }

    #[derive(Debug)]
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
        let r = tokens(CODE.chars());
        println!("{:?}", r);
        let r = r.unwrap();
        assert_eq!(
            r,
            vec![
                Token::ObjS,
                Token::Str("a".into()),
                Token::Colon,
                Token::Num(1.0),
                Token::Comma,
                Token::Str("b".into()),
                Token::Colon,
                Token::Bool(true),
                Token::Comma,
                Token::Str("c".into()),
                Token::Colon,
                Token::ArrS,
                Token::Null,
                Token::Comma,
                Token::Num(1.5),
                Token::Comma,
                Token::Bool(false),
                Token::ArrE,
                Token::Comma,
                Token::Str("d".into()),
                Token::Colon,
                Token::ObjS,
                Token::Str("v".into()),
                Token::Colon,
                Token::Str("asd".into()),
                Token::ObjE,
                Token::ObjE
            ]
        );
    }

    pub fn tokens(mut code: impl Iterator<Item = char>) -> Result<Vec<Token>, Vec<Arc<Error>>> {
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
        if errors.is_empty() {
            Ok(r)
        } else {
            Err(errors)
        }
    }

    fn root(inp: char, data: &mut TokenData, eof: bool) -> Flow {
        if eof || inp == '\0' {
            return Flow::End;
        }
        let sp = data.save();
        try_ret!(check_number(inp, sp));
        try_ret!(check_string(inp, sp));
        try_ret!(check_word(inp, sp));
        try_ret!(check_space(inp, sp));
        try_ret!(check_symbol(inp, data));
        return Error::new(TokenError::UnknownCharacter(inp, sp)).into();
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
                            return Flow::EndReDo;
                        }
                        Err(_) => {
                            let np = data.save();
                            return Error::new(TokenError::NotNum(sp..np)).into();
                        }
                    }
                } else {
                    strs.push(inp);
                    return Flow::None;
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
                    return Flow::End;
                }
                try_ret!(check_escape(inp, data.save(), &mut strs));
                strs.push(inp);
                Flow::None
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
                    return Flow::End;
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
                            return Flow::End;
                        }
                        Flow::None
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
        if first.is_alphanumeric() {
            let mut ws = vec![first];
            return move |inp: char, data: &mut TokenData, eof| -> Flow {
                if eof || inp == '\0' || !inp.is_alphanumeric() {
                    let s: String = ws.iter().collect();
                    if s == "true" {
                        data.tokens.push(Token::Bool(true));
                    } else if s == "false" {
                        data.tokens.push(Token::Bool(false));
                    } else if s == "null" {
                        data.tokens.push(Token::Null);
                    } else {
                        let np = data.save();
                        return Error::new(TokenError::UnknownWord(s, sp..np)).into();
                    }
                    return Flow::EndReDo;
                }
                ws.push(inp);
                Flow::None
            }
            .rfcall_next()
            .into();
        }
        None
    }
    #[inline]
    fn check_symbol(first: char, data: &mut TokenData) -> Option<Flow> {
        if first == ',' {
            data.tokens.push(Token::Comma)
        } else if first == ':' {
            data.tokens.push(Token::Colon)
        } else if first == '{' {
            data.tokens.push(Token::ObjS)
        } else if first == '}' {
            data.tokens.push(Token::ObjE)
        } else if first == '[' {
            data.tokens.push(Token::ArrS)
        } else if first == ']' {
            data.tokens.push(Token::ArrE)
        } else {
            return None;
        }
        Some(Flow::None)
    }
    fn check_space(first: char, _: usize) -> Option<Flow> {
        if first.is_whitespace() {
            return move |inp: char, _: &mut TokenData, eof| -> Flow {
                if eof || inp == '\0' || !inp.is_whitespace() {
                    return Flow::EndReDo;
                }
                Flow::None
            }
            .rfcall_next()
            .into();
        }
        None
    }
}
