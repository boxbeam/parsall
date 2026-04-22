use std::ops::{RangeFull, RangeInclusive};

use crate::{
    Context, Input, ParserError, ParserResult,
    error::ErrorHandler,
    output::{Ignore, Keep},
    parser_fn,
    parser_trait::{FixedLengthParser, Parser},
};

impl<E> Parser<E, ()> for RangeFull
where
    E: From<ParserError>,
{
    type Output<'a> = char;
    type Kind = Keep;

    fn parse<'a>(
        &mut self,
        input: Input<'a>,
        _errs: impl ErrorHandler<E>,
        _ctx: Context<()>,
    ) -> ParserResult<Self::Output<'a>> {
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
    E: From<ParserError>,
{
    type Output<'a> = char;
    type Kind = Keep;

    fn parse<'a>(
        &mut self,
        input: Input<'a>,
        errs: impl ErrorHandler<E>,
        _ctx: Context<()>,
    ) -> ParserResult<Self::Output<'a>> {
        let c = input.slice().chars().next();
        if let Some(c) = c
            && self.contains(&c)
        {
            Some((c.len_utf8(), c))
        } else {
            errs.error(ParserError::ExpectedSymbol(self), input.cur..input.cur);
            None
        }
    }
}

impl<E, C> Parser<E, C> for &'static str
where
    E: From<ParserError>,
{
    type Output<'b> = ();
    type Kind = Ignore;

    fn parse<'a>(
        &mut self,
        input: Input<'a>,
        errs: impl ErrorHandler<E>,
        ctx: Context<C>,
    ) -> ParserResult<Self::Output<'a>> {
        lit(self, self).parse(input, errs, ctx)
    }
}

impl<E, C> FixedLengthParser<E, C> for &'static str
where
    E: From<ParserError>,
{
    fn parsed_len(&self) -> usize {
        str::len(self)
    }
}

impl<F> Parser<ParserError, ()> for F
where
    F: Fn(char) -> bool,
{
    type Output<'a> = char;
    type Kind = Keep;

    fn parse<'a>(
        &mut self,
        input: Input<'a>,
        _errs: impl ErrorHandler<ParserError>,
        _ctx: Context<()>,
    ) -> ParserResult<Self::Output<'a>> {
        let c = input.slice().chars().next().filter(|c| self(*c))?;
        Some((c.len_utf8(), c))
    }
}

impl<E> Parser<E, ()> for char
where
    E: From<ParserError>,
{
    type Output<'a> = ();
    type Kind = Ignore;

    fn parse<'a>(
        &mut self,
        input: Input<'a>,
        errs: impl ErrorHandler<E>,
        _ctx: Context<()>,
    ) -> ParserResult<Self::Output<'a>> {
        if input.slice().starts_with(*self) {
            Some((self.len_utf8(), ()))
        } else {
            errs.error(ParserError::ExpectedChar(*self), input.cur..input.cur);
            None
        }
    }
}

impl<E> Parser<E, ()> for RangeInclusive<char>
where
    E: From<ParserError>,
{
    type Output<'a> = char;
    type Kind = Keep;

    fn parse<'a>(
        &mut self,
        input: Input<'a>,
        errs: impl ErrorHandler<E>,
        _ctx: Context<()>,
    ) -> ParserResult<Self::Output<'a>> {
        let c = input.slice().chars().next();
        if let Some(c) = c
            && self.contains(&c)
        {
            Some((c.len_utf8(), c))
        } else {
            errs.error(
                ParserError::ExpectedRange(self.clone()),
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
    E: From<ParserError>,
{
    struct LitParser {
        lit: &'static str,
        parser_name: &'static str,
    }

    impl<E, C> Parser<E, C> for LitParser
    where
        E: From<ParserError>,
    {
        type Kind = Ignore;
        type Output<'a> = ();
        fn parse(
            &mut self,
            input: Input,
            errs: impl ErrorHandler<E>,
            _ctx: Context<C>,
        ) -> ParserResult<()> {
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
                    ParserError::ExpectedLiteral(self.lit, self.parser_name),
                    input.cur..input.cur + num_matching,
                );
                None
            }
        }
    }

    impl<E, C> FixedLengthParser<E, C> for LitParser
    where
        E: From<ParserError>,
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
    E: From<ParserError>,
{
    struct Filter<F> {
        f: F,
        parser_name: &'static str,
    }
    impl<E, F, C> Parser<E, C> for Filter<F>
    where
        F: Fn(char) -> bool,
        E: From<ParserError>,
    {
        type Output<'a> = char;
        type Kind = Keep;

        fn parse(
            &mut self,
            input: Input,
            errs: impl ErrorHandler<E>,
            _ctx: Context<C>,
        ) -> ParserResult<Self::Output<'_>> {
            let next_char = input.slice().chars().next();
            if let Some(c) = next_char
                && (self.f)(c)
            {
                Some((c.len_utf8(), c))
            } else {
                errs.error(
                    ParserError::ExpectedToken(self.parser_name),
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
