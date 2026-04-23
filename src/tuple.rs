use crate::{
    output::{Chain, ChainImpl},
    prelude::Parser,
};

macro_rules! chain_output {
    ($lt:lifetime, $l:ty, $r:ty) => { <ChainImpl<<$l>::Kind, <$r>::Kind> as Chain>::Output<<$l>::Output<$lt>, <$r>::Output<$lt>>};
    ($lt:lifetime, $l:ty, $r:ty, $c:ty) => { <ChainImpl<chain_kind!($l, $r), <$c>::Kind> as Chain>::Output<chain_output!('a, $l, $r), $c> };
    ($lt:lifetime, $l:ty, $r:ty, $c:ty, $($rest:ty),+) => { chain_output!($lt, chain_output!($lt, $l, $r, $c), $($rest),+) }
}

macro_rules! chain_kind {
    ($l:ty, $r:ty) => {
        <ChainImpl<<$l>::Kind, <$r>::Kind> as Chain>::NextKind
    };
}

impl<A, B, E, C> Parser<E, C> for (A, B)
where
    A: Parser<E, C>,
    B: Parser<E, C>,
    ChainImpl<A::Kind, B::Kind>: Chain,
{
    type Output<'a> = chain_output!('a, A, B);
    type Kind = chain_kind!(A, B);

    fn parse<'a>(
        &mut self,
        input: crate::Input<'a>,
        errs: impl crate::prelude::ErrorHandler<E>,
        ctx: crate::Context<C>,
    ) -> crate::ParserResult<Self::Output<'a>> {
        let mut offset = 0;
        let (len, a) = self.0.parse(input, errs.clone(), ctx)?;
        offset += len;
        let (len, b) = self.1.parse(input, errs.clone(), ctx)?;
        let output = ChainImpl::<A::Kind, B::Kind>::chain(a, b);
        offset += len;
        Some((offset, output))
    }
}

impl<A, B, C, Err, Ctx> Parser<Err, Ctx> for (A, B, C)
where
    A: Parser<Err, Ctx>,
    B: Parser<Err, Ctx>,
    C: Parser<Err, Ctx>,
    ChainImpl<A::Kind, B::Kind>: Chain,
    ChainImpl<chain_kind!(A, B), C::Kind>: Chain,
{
    type Output<'a> = chain_output!('a, A, B, C);
    type Kind = chain_kind!(A, B);

    fn parse<'a>(
        &mut self,
        input: crate::Input<'a>,
        errs: impl crate::prelude::ErrorHandler<Err>,
        ctx: crate::Context<Ctx>,
    ) -> crate::ParserResult<Self::Output<'a>> {
        let mut offset = 0;
        let (len, a) = self.0.parse(input, errs.clone(), ctx)?;
        offset += len;
        let (len, b) = self.1.parse(input, errs.clone(), ctx)?;
        let chain = ChainImpl::<A::Kind, B::Kind>::chain(a, b);
        offset += len;
        let (len, c) = self.2.parse(input, errs.clone(), ctx)?;
        let chain = ChainImpl::<chain_kind!(A, B), C::Kind>::chain(chain, c);
        offset += len;
        Some((offset, chain))
    }
}
