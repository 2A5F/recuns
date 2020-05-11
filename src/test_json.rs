// #![allow(unused_variables, unused_mut, unused_imports, dead_code)]
use crate::*;
use anyhow::Error;
use batch_oper::*;
use lazy_static::*;
use std::collections::BTreeMap;
use std::ops::*;
use thiserror::*;
use token::*;

static CODE: &'static str =
    r#"{ "a": 1, "b": true, "c": [null, 1.5, false], "d": { "v": "asd" } }"#;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<JsonValue>),
    Obj(BTreeMap<String, JsonValue>),
}

#[derive(Error, Debug, PartialEq)]
pub enum JsonParserError {
    #[error("Need <{}> but find EOF", .0)]
    NeedButEof(String),
    #[error("Excess Token <{}>", .0)]
    ExcessToken(Token),
    #[error("Need <{}> but find <{}>", .0, .1)]
    NeedBut(String, Token),
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
    let r = r.unwrap();
    assert_eq!(
        r,
        Some(JsonValue::Obj({
            let mut m = BTreeMap::new();
            m.insert("a".into(), JsonValue::Num(1.0));
            m.insert("b".into(), JsonValue::Bool(true));
            m.insert(
                "c".into(),
                JsonValue::Arr(vec![
                    JsonValue::Null,
                    JsonValue::Num(1.5),
                    JsonValue::Bool(false),
                ]),
            );
            m.insert(
                "d".into(),
                JsonValue::Obj({
                    let mut m = BTreeMap::new();
                    m.insert("v".into(), JsonValue::Str("asd".into()));
                    m
                }),
            );
            m
        }))
    )
}

pub fn json(code: impl Iterator<Item = char>) -> Result<Option<JsonValue>, Vec<Arc<Error>>> {
    let tokens = tokens(code)?;
    let mut tokens = tokens.iter();
    let r = root.recuns();
    let r = do_loop(ParserData { out: None }, r, true, |_| {
        tokens.next().map(|v| Ok(v.clone()))
    })?;
    Ok(r.unwrap().out)
}

fn root(inp: Token, data: &mut ParserData, eof: bool) -> Flow {
    if eof {
        return Flow::End;
    }
    if data.out.is_some() {
        return Error::new(JsonParserError::ExcessToken(inp)).into();
    }
    let data: *mut ParserData = data;
    check_value(&inp, Box::new(move |v| unsafe { &mut *data }.out = Some(v)))
}
fn check_value(inp: &Token, mut cb: Box<dyn FnMut(JsonValue)>) -> Flow {
    try_ret!(check_literal(inp, |v| cb(v)));
    if let Some(f) = check_arr(inp) {
        let r = f(Box::new(move |v| cb(v)));
        return r;
    }
    if let Some(f) = check_obj(inp) {
        let r = f(Box::new(move |v| cb(v)));
        return r;
    }
    return Error::new(JsonParserError::NeedBut("value".into(), inp.clone())).into();
}
fn check_literal(inp: &Token, mut cb: impl FnMut(JsonValue)) -> Option<Flow> {
    match inp {
        Token::Str(s, _) => cb(JsonValue::Str(s.clone())),
        &Token::Num(n, _) => cb(JsonValue::Num(n)),
        &Token::Bool(b, _) => cb(JsonValue::Bool(b)),
        Token::Null(_) => cb(JsonValue::Null),
        _ => return None,
    }
    Some(Flow::None)
}
#[inline]
fn check_arr(inp: &Token) -> Option<impl FnOnce(Box<dyn FnMut(JsonValue)>) -> Flow> {
    if let Token::ArrS(_) = inp {
        return Some(|mut cb: Box<dyn FnMut(JsonValue)>| {
            let mut vals = vec![];
            let mut split = false;
            return move |inp: Token, _: &mut ParserData, eof: bool| -> Flow {
                if eof {
                    return Error::new(JsonParserError::NeedButEof("]".into())).into();
                }
                if let Token::ArrE(_) = inp {
                    cb(JsonValue::Arr(std::mem::replace(&mut vals, vec![])));
                    return Flow::End;
                }
                if split {
                    split = false;
                    if let Token::Comma(_) = inp {
                        return Flow::None;
                    } else {
                        return Error::new(JsonParserError::NeedBut(",".into(), inp)).into();
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
            .rfcall_next("check_arr");
        });
    }
    None
}
#[inline]
fn check_obj(inp: &Token) -> Option<impl FnOnce(Box<dyn FnMut(JsonValue)>) -> Flow> {
    if let Token::ObjS(_) = inp {
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
                    return match need {
                        Need::Colon => Error::new(JsonParserError::NeedButEof(":".into())).into(),
                        Need::Value => {
                            Error::new(JsonParserError::NeedButEof("value".into())).into()
                        }
                        _ => Error::new(JsonParserError::NeedButEof("]".into())).into(),
                    };
                }
                if let Token::ObjE(_) = inp {
                    cb(JsonValue::Obj(std::mem::replace(
                        &mut vals,
                        BTreeMap::new(),
                    )));
                    return Flow::End;
                }
                match need {
                    Need::Key => {
                        if let Token::Str(k, _) = inp {
                            key = Some(k);
                            need = Need::Colon;
                        } else {
                            return Error::new(JsonParserError::NeedBut("key".into(), inp)).into();
                        }
                    }
                    Need::Colon => {
                        if let Token::Colon(_) = inp {
                            need = Need::Value;
                        } else {
                            return Error::new(JsonParserError::NeedBut(":".into(), inp)).into();
                        }
                    }
                    Need::Value => {
                        let vals: *mut _ = &mut vals;
                        let key: *mut _ = &mut key;
                        let need: *mut _ = &mut need;
                        return check_value(
                            &inp,
                            Box::new(move |v| unsafe {
                                let k = std::mem::replace(&mut *key, None).unwrap();
                                (&mut *vals).insert(k, v);
                                *need = Need::Comma;
                            }),
                        );
                    }
                    Need::Comma => {
                        if let Token::Comma(_) = inp {
                            need = Need::Key;
                        } else {
                            return Error::new(JsonParserError::NeedBut(",".into(), inp)).into();
                        }
                    }
                }
                Flow::None
            }
            .rfcall_next("check_obj");
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
        Str(String, Range<usize>),
        Num(f64, Range<usize>),
        Bool(bool, Range<usize>),
        Null(Range<usize>),
        /// `[`
        ArrS(Range<usize>),
        /// `]`
        ArrE(Range<usize>),
        /// `{`
        ObjS(Range<usize>),
        /// `}`
        ObjE(Range<usize>),
        /// `,`
        Comma(Range<usize>),
        /// `:`
        Colon(Range<usize>),
    }
    impl Default for Token {
        fn default() -> Self {
            Self::None
        }
    }
    impl std::fmt::Display for Token {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Token::None => write!(f, "None"),
                Token::Str(s, r) => write!(f, "Str({}) at {}..{}", s, r.start, r.end),
                Token::Num(n, r) => write!(f, "Num({}) at {}..{}", n, r.start, r.end),
                Token::Bool(b, r) => write!(f, "Bool({}) at {}..{}", b, r.start, r.end),
                Token::Null(r) => write!(f, "Null(null) at {}..{}", r.start, r.end),
                Token::ArrS(r) => write!(f, "ArrS('[') at {}..{}", r.start, r.end),
                Token::ArrE(r) => write!(f, "ArrE(']') at {}..{}", r.start, r.end),
                Token::ObjS(r) => write!(f, "ObjS('{{') at {}..{}", r.start, r.end),
                Token::ObjE(r) => write!(f, "ObjE('}}') at {}..{}", r.start, r.end),
                Token::Comma(r) => write!(f, "Comma(',') at {}..{}", r.start, r.end),
                Token::Colon(r) => write!(f, "Colon(':') at {}..{}", r.start, r.end),
            }
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
                Token::ObjS(1..1),
                Token::Str("a".into(), 3..5),
                Token::Colon(6..6),
                Token::Num(1.0, 8..8),
                Token::Comma(9..9),
                Token::Str("b".into(), 11..13),
                Token::Colon(14..14),
                Token::Bool(true, 16..19),
                Token::Comma(20..20),
                Token::Str("c".into(), 22..24),
                Token::Colon(25..25),
                Token::ArrS(27..27),
                Token::Null(28..31),
                Token::Comma(32..32),
                Token::Num(1.5, 34..36),
                Token::Comma(37..37),
                Token::Bool(false, 39..43),
                Token::ArrE(44..44),
                Token::Comma(45..45),
                Token::Str("d".into(), 47..49),
                Token::Colon(50..50),
                Token::ObjS(52..52),
                Token::Str("v".into(), 54..56),
                Token::Colon(57..57),
                Token::Str("asd".into(), 59..63),
                Token::ObjE(65..65),
                Token::ObjE(67..67)
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
        try_ret!(check_symbol(inp, data, sp));
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
                    let np = data.save() - 1;
                    if !number_regex.is_match(&*s) {
                        return Error::new(TokenError::NotNum(sp..np)).into();
                    }
                    let f = s.parse::<f64>();
                    match f {
                        Ok(f) => {
                            data.tokens.push(Token::Num(f, sp..np));
                            return Flow::EndReDo;
                        }
                        Err(_) => {
                            return Error::new(TokenError::NotNum(sp..np)).into();
                        }
                    }
                } else {
                    strs.push(inp);
                    return Flow::None;
                }
            }
            .rfcall_next("check_number")
            .into();
        }
        None
    }
    fn check_string(first: char, sp: usize) -> Option<Flow> {
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
                    let np = data.save();
                    data.tokens.push(Token::Str(s, sp..np));
                    return Flow::End;
                }
                try_ret!(check_escape(inp, data.save(), &mut strs));
                strs.push(inp);
                Flow::None
            }
            .rfcall_next("check_string")
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
                    .rfmov_next("check_escape_unicode".into());
                } else {
                    let np = data.save();
                    return Error::new(TokenError::IllegalEscape(inp, np)).into();
                }
            }
            .rfcall_next("check_escape")
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
                    let np = data.save() - 1;
                    if s == "true" {
                        data.tokens.push(Token::Bool(true, sp..np));
                    } else if s == "false" {
                        data.tokens.push(Token::Bool(false, sp..np));
                    } else if s == "null" {
                        data.tokens.push(Token::Null(sp..np));
                    } else {
                        return Error::new(TokenError::UnknownWord(s, sp..np)).into();
                    }
                    return Flow::EndReDo;
                }
                ws.push(inp);
                Flow::None
            }
            .rfcall_next("check_word")
            .into();
        }
        None
    }
    #[inline]
    fn check_symbol(first: char, data: &mut TokenData, sp: usize) -> Option<Flow> {
        let np = data.save();
        if first == ',' {
            data.tokens.push(Token::Comma(sp..np))
        } else if first == ':' {
            data.tokens.push(Token::Colon(sp..np))
        } else if first == '{' {
            data.tokens.push(Token::ObjS(sp..np))
        } else if first == '}' {
            data.tokens.push(Token::ObjE(sp..np))
        } else if first == '[' {
            data.tokens.push(Token::ArrS(sp..np))
        } else if first == ']' {
            data.tokens.push(Token::ArrE(sp..np))
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
            .rfcall_next("check_space")
            .into();
        }
        None
    }
}
