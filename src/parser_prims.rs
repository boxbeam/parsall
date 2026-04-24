use std::ops::{RangeFull, RangeInclusive};

use crate::{
    Context, Input, ParseError, ParseResult,
    error::ErrorHandler,
    output::{Ignore, Keep},
    parser_fn,
    parser_trait::{FixedLengthParser, Parser},
};

impl<E> Parser<E, ()> for RangeFull
where
    E: From<ParseError>,
{
    type Output<'a> = char;
    type Kind = Keep;

    fn parse<'a, H>(
        &mut self,
        input: Input<'a>,
        _errs: H,
        _ctx: Context<()>,
    ) -> ParseResult<Self::Output<'a>>
    where
        H: ErrorHandler,
        H::Err: From<E>,
    {
        let c = input.slice().chars().next();
        if let Some(c) = c {
            Some((c.len_utf8(), c))
        } else {
            None
        }
    }
}

impl<E> Parser<E, ()> for &'static [char]
where
    E: From<ParseError>,
{
    type Output<'a> = char;
    type Kind = Keep;

    fn parse<'a, H>(
        &mut self,
        input: Input<'a>,
        errs: H,
        _ctx: Context<()>,
    ) -> ParseResult<Self::Output<'a>>
    where
        H: ErrorHandler,
        H::Err: From<E>,
    {
        let c = input.slice().chars().next();
        if let Some(c) = c
            && self.contains(&c)
        {
            Some((c.len_utf8(), c))
        } else {
            errs.error(
                E::from(ParseError::ExpectedSymbol(self)),
                input.cur..input.cur,
            );
            None
        }
    }
}

impl<E, C> Parser<E, C> for &'static str
where
    E: From<ParseError>,
{
    type Output<'b> = ();
    type Kind = Ignore;

    fn parse<'a, H>(
        &mut self,
        input: Input<'a>,
        errs: H,
        ctx: Context<C>,
    ) -> ParseResult<Self::Output<'a>>
    where
        H: ErrorHandler,
        H::Err: From<E>,
    {
        lit(self, self).parse(input, errs, ctx)
    }
}

impl<E, C> FixedLengthParser<E, C> for &'static str
where
    E: From<ParseError>,
{
    fn parsed_len(&self) -> usize {
        str::len(self)
    }
}

impl<F, E> Parser<E, ()> for F
where
    F: Fn(char) -> bool,
    E: From<ParseError>,
{
    type Output<'a> = char;
    type Kind = Keep;

    fn parse<'a, H>(
        &mut self,
        input: Input<'a>,
        _errs: H,
        _ctx: Context<()>,
    ) -> ParseResult<Self::Output<'a>>
    where
        H: ErrorHandler,
        H::Err: From<E>,
    {
        let c = input.slice().chars().next().filter(|c| self(*c))?;
        Some((c.len_utf8(), c))
    }
}

impl<E, C> Parser<E, C> for char
where
    E: From<ParseError>,
{
    type Output<'a> = ();
    type Kind = Ignore;

    fn parse<'a, H>(
        &mut self,
        input: Input<'a>,
        errs: H,
        _ctx: Context<C>,
    ) -> ParseResult<Self::Output<'a>>
    where
        H: ErrorHandler,
        H::Err: From<E>,
    {
        if input.slice().starts_with(*self) {
            Some((self.len_utf8(), ()))
        } else {
            errs.error(
                E::from(ParseError::ExpectedChar(*self)),
                input.cur..input.cur,
            );
            None
        }
    }
}

impl<E, C> FixedLengthParser<E, C> for char
where
    E: From<ParseError>,
{
    fn parsed_len(&self) -> usize {
        self.len_utf8()
    }
}

impl<E> Parser<E, ()> for RangeInclusive<char>
where
    E: From<ParseError>,
{
    type Output<'a> = char;
    type Kind = Keep;

    fn parse<'a, H>(
        &mut self,
        input: Input<'a>,
        errs: H,
        _ctx: Context<()>,
    ) -> ParseResult<Self::Output<'a>>
    where
        H: ErrorHandler,
        H::Err: From<E>,
    {
        let c = input.slice().chars().next();
        if let Some(c) = c
            && self.contains(&c)
        {
            Some((c.len_utf8(), c))
        } else {
            errs.error(
                E::from(ParseError::ExpectedRange(self.clone())),
                input.cur..input.cur,
            );
            None
        }
    }
}

fn lit<E, C>(
    lit: &'static str,
    parser_name: &'static str,
) -> impl for<'a> FixedLengthParser<E, C, Output<'a> = (), Kind = Ignore>
where
    E: From<ParseError>,
{
    struct LitParser {
        lit: &'static str,
        parser_name: &'static str,
    }

    impl<E, C> Parser<E, C> for LitParser
    where
        E: From<ParseError>,
    {
        type Kind = Ignore;
        type Output<'a> = ();
        fn parse<'a, H>(&mut self, input: Input, errs: H, _ctx: Context<C>) -> ParseResult<()>
        where
            H: ErrorHandler,
            H::Err: From<E>,
        {
            let num_matching = input
                .slice()
                .bytes()
                .zip(self.lit.bytes())
                .take_while(|(a, b)| a == b)
                .count();
            if num_matching == self.lit.len() {
                Some((self.lit.len(), ()))
            } else {
                errs.error(
                    E::from(ParseError::ExpectedLiteral(self.lit, self.parser_name)),
                    input.cur..input.cur + num_matching,
                );
                None
            }
        }
    }

    impl<E, C> FixedLengthParser<E, C> for LitParser
    where
        E: From<ParseError>,
    {
        fn parsed_len(&self) -> usize {
            self.lit.len()
        }
    }

    LitParser { lit, parser_name }
}

pub fn char_filter<E>(
    filter: impl Fn(char) -> bool,
    parser_name: &'static str,
) -> impl for<'a> Parser<E, Output<'a> = char, Kind = Keep>
where
    E: From<ParseError>,
{
    struct Filter<F> {
        f: F,
        parser_name: &'static str,
    }
    impl<E, F, C> Parser<E, C> for Filter<F>
    where
        F: Fn(char) -> bool,
        E: From<ParseError>,
    {
        type Output<'a> = char;
        type Kind = Keep;

        fn parse<'a, H>(
            &mut self,
            input: Input,
            errs: H,
            _ctx: Context<C>,
        ) -> ParseResult<Self::Output<'_>>
        where
            H: ErrorHandler,
            H::Err: From<E>,
        {
            let next_char = input.slice().chars().next();
            if let Some(c) = next_char
                && (self.f)(c)
            {
                Some((c.len_utf8(), c))
            } else {
                errs.error(
                    E::from(ParseError::ExpectedToken(self.parser_name)),
                    input.cur..input.cur,
                );
                None
            }
        }
    }
    Filter {
        f: filter,
        parser_name,
    }
}

parser_fn!(pub sep((|c: char| c.is_whitespace()).rep(Ignore).opt()));
