use crate::error::{ErrorLocation, ParseError};

pub mod error;
pub mod macros;
pub mod output;
pub mod parser_prims;
pub mod parser_trait;

pub mod prelude {
    pub use crate::{
        Context, MatchResult, ParserResult, error::*, macros::*, output::*, parser_prims::*,
        parser_trait::*,
    };
}

pub type ParserResult<T> = Option<(usize, T)>;
pub type Context<'a, T> = &'a mut T;
pub type MatchResult<T, E> = Result<T, Option<ErrorLocation<E>>>;

#[derive(Clone, Copy)]
pub struct Input<'a> {
    pub src: &'a str,
    pub cur: usize,
}

impl<'a> Input<'a> {
    pub fn slice(&'a self) -> &'a str {
        &self.src[self.cur..]
    }
}

impl<'a> From<&'a str> for Input<'a> {
    fn from(value: &'a str) -> Self {
        Input { src: value, cur: 0 }
    }
}

impl<'a> Input<'a> {
    pub fn skip(self, len: usize) -> Self {
        Input {
            cur: self.cur + len,
            ..self
        }
    }
}
