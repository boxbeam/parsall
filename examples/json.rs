#![allow(unused)]

use parsall::{plookahead, prelude::*};
use std::{
    collections::HashMap,
    num::{ParseFloatError, ParseIntError},
};

#[derive(Debug, PartialEq)]
pub enum JSONValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
    List(Vec<JSONValue>),
    Map(HashMap<String, JSONValue>),
}

parser_fns! {
    StrChar((['"', '\\'].not(), ..)) -> char;

    EscapeSeq('\\', ..) -> char;

    Str('"', EscapeSeq.or(StrChar).rep(Ignore).slice().map(ToOwned::to_owned), '"') -> String;

    Null("null") -> JSONValue { JSONValue::Null };

    Digits(('0'..='9').rep(Ignore));

    Int(("-".opt(), Digits).slice().try_map(str::parse).map(JSONValue::Int)) -> JSONValue, JSONError;
    Float(("-".opt(), Digits, ".", Digits).slice().try_map(str::parse).map(JSONValue::Float)) -> JSONValue, JSONError;

    Bool(pmatch!{
        "true" => JSONValue::Bool(true),
        "false" => JSONValue::Bool(false),
    }) -> JSONValue;

    List(Value.delim_by(",".pad(sep), ToVec).opt_default().pad(sep).wrapped("[", "]").map(JSONValue::List)) -> JSONValue, JSONError;

    MapEntry((Str, ":".pad(sep), Value)) -> (String, JSONValue), JSONError;
    Map(MapEntry.delim_by(",".pad(sep), collect()).opt_default().map(JSONValue::Map).pad(sep).wrapped("{", "}")) -> JSONValue, JSONError;

    Never("".not());
    JStr(Str.map(JSONValue::String)) -> JSONValue;
    IntOrFloat(Float.or(Int)) -> JSONValue, JSONError;
    // Overflows when Map or List
    // Overflows when _Lookahead (inlined and modified plookahead! invocation)
    // Does not overflow when Map.or(List)
    // Does overflow on Map.then("")
    // Map.or(Never) works?
    // Value(Value) breaks, this seems obvious to crash at runtime but why at comptime?
    pub Value("".then(Value)) -> JSONValue, JSONError;
}

fn thing() {}

#[derive(Debug)]
enum JSONError {
    Int(ParseIntError),
    Float(ParseFloatError),
    Parse(ParseError),
}

impl From<ParseIntError> for JSONError {
    fn from(value: ParseIntError) -> Self {
        JSONError::Int(value)
    }
}

impl From<ParseFloatError> for JSONError {
    fn from(value: ParseFloatError) -> Self {
        JSONError::Float(value)
    }
}

impl From<ParseError> for JSONError {
    fn from(value: ParseError) -> Self {
        JSONError::Parse(value)
    }
}

fn main() {
    Parser::<JSONError>::try_match(&mut Value, include_str!("/home/june/Downloads/big.json"))
        .unwrap();
}
