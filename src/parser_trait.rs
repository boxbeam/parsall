use std::{io::Write, marker::PhantomData};

use crate::{
    Context, ErrorLocation, Input, ParseError, ParseResult,
    error::{ErrorCell, ErrorHandler},
    output::{ChainMode, Collector, DelimitedCollector, Ignore, Keep, OptionalOutput},
};

pub trait Parser<Err = ParseError, Ctx = ()> {
    type Output<'a>;
    type Kind: ChainMode;

    fn parse<'a, H>(
        &mut self,
        input: Input<'a>,
        errs: H,
        ctx: Context<Ctx>,
    ) -> ParseResult<Self::Output<'a>>
    where
        H: ErrorHandler,
        H::Err: From<Err>;

    fn try_match<'a>(
        &mut self,
        input: &'a str,
    ) -> Result<Self::Output<'a>, Option<ErrorLocation<Err>>>
    where
        Ctx: Default,
        Err: From<ParseError>,
    {
        let input = Input { src: input, cur: 0 };
        let mut ctx = Default::default();
        let errs = ErrorCell::default();
        match self.parse(input, &errs, &mut ctx) {
            Some((_, e)) => Ok(e),
            None => Err(errs.into_inner()),
        }
    }

    #[inline(always)]
    fn repl<O>(mut self)
    where
        Self: Sized + for<'a> Parser<Err, Ctx, Output<'a> = O>,
        Ctx: Default,
        Err: std::fmt::Debug + From<ParseError>,
        O: std::fmt::Debug,
    {
        print!("> ");
        std::io::stdout().flush().unwrap();
        for line in std::io::stdin().lines() {
            let line = line.unwrap();
            let result = self.try_match(&line);
            println!("\n{result:?}");
            print!("\n> ");
            std::io::stdout().flush().unwrap();
        }
    }

    #[inline(always)]
    fn rep<Coll, K>(
        self,
        coll: Coll,
    ) -> impl for<'a> Parser<
        Err,
        Ctx,
        Output<'a> = <Coll as Collector<Self::Output<'a>>>::Container,
        Kind = K,
    >
    where
        Self: Sized,
        Coll: for<'a> Collector<Self::Output<'a>, Kind = K>,
        K: ChainMode,
    {
        struct Repeat<P, Coll, K> {
            p: P,
            coll: Coll,
            phantom: PhantomData<K>,
        }

        impl<P, E, C, Coll, K> Parser<E, C> for Repeat<P, Coll, K>
        where
            P: Parser<E, C>,
            Coll: for<'a> Collector<P::Output<'a>, Kind = K>,
            K: ChainMode,
        {
            type Kind = K;
            type Output<'a> = <Coll as Collector<P::Output<'a>>>::Container;
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
                let (mut offset, _first) = self.p.parse(input, errs.clone(), ctx)?;
                let mut elems = Coll::Container::default();
                while let Some((len, elem)) = self.p.parse(input.skip(offset), errs.clone(), ctx) {
                    offset += len;
                    self.coll.consume(&mut elems, elem);
                }
                Some((offset, elems))
            }
        }

        Repeat {
            p: self,
            coll,
            phantom: PhantomData,
        }
    }

    #[inline(always)]
    fn or<P>(
        self,
        other: P,
    ) -> impl for<'a> Parser<Err, Ctx, Output<'a> = Self::Output<'a>, Kind = Self::Kind>
    where
        Self: Sized,
        P: for<'a> Parser<Err, Ctx, Output<'a> = Self::Output<'a>>,
    {
        struct Or<P1, P2> {
            l: P1,
            r: P2,
        }

        impl<P1, P2, E, C> Parser<E, C> for Or<P1, P2>
        where
            P1: for<'a> Parser<E, C, Output<'a> = P2::Output<'a>>,
            P2: Parser<E, C>,
        {
            type Kind = P1::Kind;
            type Output<'a> = P1::Output<'a>;
            fn parse<'a, H>(
                &mut self,
                input: Input<'a>,
                errs: H,
                ctx: Context<C>,
            ) -> ParseResult<P1::Output<'a>>
            where
                H: ErrorHandler,
                H::Err: From<E>,
            {
                let err = ErrorCell::<H::Err>::default();
                let parsed = self.l.parse(input, &err, ctx);
                if parsed.is_some() {
                    return parsed;
                }
                match self.r.parse(input, &err, ctx) {
                    Some(val) => Some(val),
                    None => {
                        if let Some(ErrorLocation(err, pos)) = err.into_inner() {
                            errs.error(err, pos);
                        }
                        None
                    }
                }
            }
        }

        Or { l: self, r: other }
    }

    #[inline(always)]
    fn map<F, V>(self, f: F) -> impl for<'a> Parser<Err, Ctx, Output<'a> = V, Kind = Keep>
    where
        Self: Sized,
        F: for<'a> Fn(Self::Output<'a>) -> V,
    {
        struct Map<P, F> {
            p: P,
            f: F,
        }

        impl<P, F, V, E, C> Parser<E, C> for Map<P, F>
        where
            P: Parser<E, C>,
            F: for<'a> Fn(P::Output<'a>) -> V,
        {
            type Kind = Keep;
            type Output<'a> = V;
            fn parse<'a, H>(&mut self, input: Input<'a>, errs: H, ctx: Context<C>) -> ParseResult<V>
            where
                H: ErrorHandler,
                H::Err: From<E>,
            {
                self.p
                    .parse(input, errs, ctx)
                    .map(|(len, val)| (len, (self.f)(val)))
            }
        }

        Map { p: self, f }
    }

    #[inline(always)]
    fn try_map<F, V, E>(
        self,
        f: F,
    ) -> impl for<'a> Parser<Err, Ctx, Output<'a> = V, Kind = Self::Kind>
    where
        Self: Sized,
        F: for<'a> Fn(Self::Output<'a>) -> Result<V, E>,
        Err: From<E>,
    {
        struct TryMap<P, F> {
            p: P,
            f: F,
        }

        impl<P, F, V, E, E2, C> Parser<E, C> for TryMap<P, F>
        where
            P: Parser<E, C>,
            F: for<'a> Fn(P::Output<'a>) -> Result<V, E2>,
            E: From<E2>,
        {
            type Kind = P::Kind;
            type Output<'a> = V;
            fn parse<'a, H>(&mut self, input: Input, errs: H, ctx: Context<C>) -> ParseResult<V>
            where
                H: ErrorHandler,
                H::Err: From<E>,
            {
                let (len, result) = self.p.parse(input, errs.clone(), ctx)?;
                let result = (self.f)(result);
                match result {
                    Ok(v) => Some((len, v)),
                    Err(e) => {
                        let e: E = e.into();
                        errs.error(e, input.cur..input.cur + len);
                        None
                    }
                }
            }
        }

        TryMap { p: self, f }
    }

    #[inline(always)]
    fn opt(
        self,
    ) -> impl for<'a> Parser<
        Err,
        Ctx,
        Output<'a> = <Self::Kind as OptionalOutput>::Output<Option<Self::Output<'a>>>,
        Kind = Self::Kind,
    >
    where
        Self: Sized,
        Self::Kind: OptionalOutput,
    {
        struct Optional<P> {
            p: P,
        }
        impl<P, Err, Ctx> Parser<Err, Ctx> for Optional<P>
        where
            P: Parser<Err, Ctx>,
            P::Kind: OptionalOutput,
        {
            type Output<'a> = <P::Kind as OptionalOutput>::Output<Option<P::Output<'a>>>;
            type Kind = P::Kind;

            fn parse<'a, H>(
                &mut self,
                input: Input<'a>,
                errs: H,
                ctx: Context<Ctx>,
            ) -> ParseResult<Self::Output<'a>>
            where
                H: ErrorHandler,
                H::Err: From<Err>,
            {
                match self.p.parse(input, errs, ctx) {
                    Some((len, elem)) => Some((len, P::Kind::convert(Some(elem)))),
                    None => Some((0, P::Kind::convert(None))),
                }
            }
        }

        Optional { p: self }
    }

    #[inline(always)]
    fn opt_default<O>(self) -> impl for<'a> Parser<Err, Ctx, Output<'a> = O, Kind = Self::Kind>
    where
        Self: Sized + for<'a> Parser<Err, Ctx, Output<'a> = O>,
        O: Default,
    {
        struct OptDefault<P, O> {
            p: P,
            phantom: PhantomData<O>,
        }
        impl<P, Err, Ctx, O> Parser<Err, Ctx> for OptDefault<P, O>
        where
            P: for<'a> Parser<Err, Ctx, Output<'a> = O>,
            O: Default,
        {
            type Output<'a> = O;
            type Kind = P::Kind;

            fn parse<'a, H>(
                &mut self,
                input: Input<'a>,
                errs: H,
                ctx: Context<Ctx>,
            ) -> ParseResult<Self::Output<'a>>
            where
                H: ErrorHandler,
                H::Err: From<Err>,
            {
                match self.p.parse(input, errs, ctx) {
                    Some((len, elem)) => Some((len, elem)),
                    None => Some((0, O::default())),
                }
            }
        }

        OptDefault {
            p: self,
            phantom: PhantomData,
        }
    }

    #[inline(always)]
    fn drop(self) -> impl for<'a> Parser<Err, Ctx, Output<'a> = (), Kind = Ignore>
    where
        Self: Sized,
    {
        struct Drop<P> {
            p: P,
        }
        impl<P, E, C> Parser<E, C> for Drop<P>
        where
            P: Parser<E, C>,
        {
            type Output<'a> = ();
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
                let (len, _) = self.p.parse(input, errs, ctx)?;
                Some((len, ()))
            }
        }
        Drop { p: self }
    }

    #[inline(always)]
    fn keep(self) -> impl for<'a> Parser<Err, Ctx, Output<'a> = Self::Output<'a>, Kind = Keep>
    where
        Self: Sized,
    {
        struct KeepP<P> {
            p: P,
        }
        impl<P, E, C> Parser<E, C> for KeepP<P>
        where
            P: Parser<E, C>,
        {
            type Output<'a> = P::Output<'a>;
            type Kind = Keep;

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
                let (len, val) = self.p.parse(input, errs, ctx)?;
                Some((len, val))
            }
        }
        KeepP { p: self }
    }

    fn slice(self) -> impl for<'a> Parser<Err, Ctx, Output<'a> = &'a str, Kind = Keep>
    where
        Self: Sized,
    {
        struct Slice<P> {
            p: P,
        }
        impl<P, E, C> Parser<E, C> for Slice<P>
        where
            P: Parser<E, C>,
        {
            type Output<'a> = &'a str;
            type Kind = Keep;

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
                let (len, _) = self.p.parse(input, errs, ctx)?;
                let slice = &input.src[input.cur..input.cur + len];
                Some((len, slice))
            }
        }
        Slice { p: self }
    }

    #[inline(always)]
    fn delim_by<P, Coll, K>(
        self,
        delim: P,
        collect: Coll,
    ) -> impl for<'a> Parser<
        Err,
        Ctx,
        Output<'a> = <Coll as DelimitedCollector<Self::Output<'a>, P::Output<'a>>>::Container,
        Kind = K,
    >
    where
        P: Parser<Err, Ctx>,
        Self: Sized,
        Coll: for<'a> DelimitedCollector<Self::Output<'a>, P::Output<'a>, Kind = K>,
        K: ChainMode,
    {
        struct DelimBy<P1, P2, Coll, K> {
            elem: P1,
            delim: P2,
            coll: Coll,
            phantom: PhantomData<K>,
        }

        impl<P1, P2, Coll, E, C, K> Parser<E, C> for DelimBy<P1, P2, Coll, K>
        where
            P1: Parser<E, C>,
            P2: Parser<E, C>,
            Coll: for<'a> DelimitedCollector<P1::Output<'a>, P2::Output<'a>, Kind = K>,
            K: ChainMode,
        {
            type Output<'a> =
                <Coll as DelimitedCollector<P1::Output<'a>, P2::Output<'a>>>::Container;
            type Kind = K;

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
                let (mut offset, first) = self.elem.parse(input, errs.clone(), ctx)?;
                let mut container = self.coll.from(first);
                while let Some((delim_len, delim)) =
                    self.delim.parse(input.skip(offset), errs.clone(), ctx)
                {
                    match self
                        .elem
                        .parse(input.skip(offset + delim_len), errs.clone(), ctx)
                    {
                        Some((elem_len, next)) => {
                            offset += delim_len + elem_len;
                            container = self.coll.consume(container, delim, next);
                        }
                        None => break,
                    }
                }
                Some((offset, container))
            }
        }

        DelimBy {
            elem: self,
            delim,
            coll: collect,
            phantom: PhantomData,
        }
    }

    #[inline(always)]
    fn then<P>(
        self,
        other: P,
    ) -> impl for<'a> Parser<
        Err,
        Ctx,
        Output<'a> = <Self::Kind as ChainMode>::Output<P::Kind, Self::Output<'a>, P::Output<'a>>,
        Kind = <Self::Kind as ChainMode>::NextKind<P::Kind>,
    >
    where
        Self: Sized,
        P: Parser<Err, Ctx>,
    {
        struct Then<P1, P2> {
            l: P1,
            r: P2,
        }

        impl<P1, P2, E, C> Parser<E, C> for Then<P1, P2>
        where
            P1: Parser<E, C>,
            P2: Parser<E, C>,
        {
            type Kind = <P1::Kind as ChainMode>::NextKind<P2::Kind>;
            type Output<'a> =
                <P1::Kind as ChainMode>::Output<P2::Kind, P1::Output<'a>, P2::Output<'a>>;
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
                let (l_len, l_val) = self.l.parse(input, errs.clone(), ctx)?;
                let (r_len, r_val) = self.r.parse(input.skip(l_len), errs, ctx)?;
                let output = P1::Kind::chain(l_val, r_val);
                Some((l_len + r_len, output))
            }
        }

        Then { l: self, r: other }
    }

    #[inline(always)]
    fn with_context<'a, C>(
        self,
        ctx: Ctx,
    ) -> impl Parser<Err, (), Output<'a> = Self::Output<'a>, Kind = Self::Kind>
    where
        Self: Sized,
    {
        struct WithContext<P, C> {
            p: P,
            c: C,
        }

        impl<P, E, C> Parser<E, ()> for WithContext<P, C>
        where
            P: Parser<E, C>,
        {
            type Output<'a> = P::Output<'a>;
            type Kind = P::Kind;

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
                self.p.parse(input, errs, &mut self.c)
            }
        }

        WithContext { p: self, c: ctx }
    }

    #[inline(always)]
    fn wrapped(
        self,
        before: &'static str,
        after: &'static str,
    ) -> impl for<'a> Parser<Err, Ctx, Output<'a> = Self::Output<'a>, Kind = Self::Kind>
    where
        Self: Sized,
        Err: From<ParseError>,
    {
        struct Wrapped<P> {
            p: P,
            before: &'static str,
            after: &'static str,
        }

        impl<P, E, C> Parser<E, C> for Wrapped<P>
        where
            P: Parser<E, C>,
            E: From<ParseError>,
        {
            type Output<'a> = P::Output<'a>;
            type Kind = P::Kind;

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
                Parser::parse(&mut self.before, input, errs.clone(), &mut ())?;
                let (len, val) = self
                    .p
                    .parse(input.skip(self.before.len()), errs.clone(), ctx)?;
                Parser::parse(
                    &mut self.after,
                    input.skip(self.before.len() + len),
                    errs.clone(),
                    &mut (),
                )?;
                Some((self.before.len() + self.after.len() + len, val))
            }
        }
        Wrapped {
            p: self,
            before,
            after,
        }
    }

    #[inline(always)]
    fn lookahead(
        self,
    ) -> impl for<'a> Parser<Err, Ctx, Output<'a> = Self::Output<'a>, Kind = Ignore>
    where
        Self: Sized,
    {
        struct Lookahead<P> {
            p: P,
        }
        impl<P, E, C> Parser<E, C> for Lookahead<P>
        where
            P: Parser<E, C>,
        {
            type Output<'a> = P::Output<'a>;
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
                let (_len, val) = self.p.parse(input, errs, ctx)?;
                Some((0, val))
            }
        }

        Lookahead { p: self }
    }

    fn not(self) -> impl for<'a> Parser<Err, Ctx, Output<'a> = (), Kind = Ignore>
    where
        Self: Sized,
        Err: From<ParseError>,
    {
        struct Not<P> {
            p: P,
        }
        impl<P, E, C> Parser<E, C> for Not<P>
        where
            P: Parser<E, C>,
            E: From<ParseError>,
        {
            type Output<'a> = ();
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
                let result = self.p.parse(input, errs.clone(), ctx);
                if result.is_some() {
                    errs.error(E::from(ParseError::UnexpectedToken), input.cur..input.cur);
                    None
                } else {
                    Some((0, ()))
                }
            }
        }

        Not { p: self }
    }

    #[inline(always)]
    fn pad<P>(
        self,
        pad: P,
    ) -> impl for<'a> Parser<Err, Ctx, Output<'a> = Self::Output<'a>, Kind = Self::Kind>
    where
        P: Parser<Err, Ctx>,
        Self: Sized,
    {
        struct Pad<P1, P2> {
            elem: P1,
            pad: P2,
        }
        impl<P1, P2, E, C> Parser<E, C> for Pad<P1, P2>
        where
            P1: Parser<E, C>,
            P2: Parser<E, C>,
        {
            type Output<'a> = P1::Output<'a>;

            type Kind = P1::Kind;

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
                let (len, _) = self.pad.parse(input, errs.clone(), ctx)?;
                let mut offset = len;
                let (len, val) = self.elem.parse(input.skip(offset), errs.clone(), ctx)?;
                offset += len;
                let (len, _) = self.pad.parse(input.skip(offset), errs, ctx)?;
                offset += len;
                Some((offset, val))
            }
        }

        Pad { elem: self, pad }
    }
}

pub trait FixedLengthParser<E, C>: Parser<E, C> {
    fn parsed_len(&self) -> usize;

    fn lookbehind(self) -> impl for<'a> Parser<E, C, Output<'a> = (), Kind = Ignore>
    where
        Self: Sized,
    {
        struct Lookbehind<P> {
            p: P,
        }
        impl<P, E, C> Parser<E, C> for Lookbehind<P>
        where
            P: FixedLengthParser<E, C>,
        {
            type Output<'a> = ();
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
                let new_input = Input {
                    src: input.src,
                    cur: input.cur - self.p.parsed_len(),
                };
                let _ = self.p.parse(new_input, errs, ctx)?;
                Some((0, ()))
            }
        }

        Lookbehind { p: self }
    }

    fn negative_lookbehind(self) -> impl for<'a> Parser<E, C, Output<'a> = (), Kind = Ignore>
    where
        Self: Sized,
        E: From<ParseError>,
    {
        struct NegativeLookbehind<P> {
            p: P,
        }
        impl<P, E, C> Parser<E, C> for NegativeLookbehind<P>
        where
            P: FixedLengthParser<E, C>,
            E: From<ParseError>,
        {
            type Output<'a> = ();
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
                let new_input = Input {
                    src: input.src,
                    cur: input.cur - self.p.parsed_len(),
                };
                let result = self.p.parse(new_input, errs.clone(), ctx);
                if result.is_some() {
                    errs.error(E::from(ParseError::UnexpectedToken), input.cur..input.cur);
                    None
                } else {
                    Some((0, ()))
                }
            }
        }

        NegativeLookbehind { p: self }
    }
}
