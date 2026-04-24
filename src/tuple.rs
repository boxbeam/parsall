use crate::{error::ErrorHandler, output::ChainMode, prelude::Parser};

struct MutRefParser<'a, P>(&'a mut P);

impl<'a, P, E, C> Parser<E, C> for MutRefParser<'a, P>
where
    P: Parser<E, C>,
{
    type Output<'b> = P::Output<'b>;

    type Kind = P::Kind;

    fn parse<'b, H>(
        &mut self,
        input: crate::Input<'b>,
        errs: H,
        ctx: crate::Context<C>,
    ) -> crate::ParseResult<Self::Output<'b>>
    where
        H: ErrorHandler,
        H::Err: From<E>,
    {
        self.0.parse(input, errs, ctx)
    }
}

macro_rules! chain_type {
    ($lt:lifetime, $ty:ty) => { <$ty>::Output<$lt> };
    ($lt:lifetime, $l:ty, $($rest:ty),+) => { <<$l>::Kind as ChainMode>::Output<chain_kind!($($rest),+), <$l>::Output<$lt>, chain_type!($lt, $($rest),+)> };
}

macro_rules! chain_kind {
    ($ty:ty) => { <$ty>::Kind };
    ($ty:ty, $($rest:ty),+) => { <<$ty>::Kind as ChainMode>::NextKind<chain_kind!($($rest),+)> }
}

macro_rules! chain_parser {
    ($first:expr) => {
        MutRefParser(&mut $first)
    };
    ($first:expr, $($rest:expr),+) => { MutRefParser(&mut $first).then(chain_parser!($($rest),+)) };
}

macro_rules! impl_parser_tuple {
    ($($p:ident),+ ; $name:ident = $($expr:expr),+) => {
        impl<Err, Ctx, $($p),+> Parser<Err, Ctx> for ($($p),+)
        where
            $($p: Parser<Err, Ctx>),+
        {
            type Output<'a> = chain_type!('a, $($p),+);
            type Kind = chain_kind!($($p),+);

            fn parse<'a, Handler>(
                &mut self,
                input: crate::Input<'a>,
                errs: Handler,
                ctx: crate::Context<Ctx>,
            ) -> crate::ParseResult<Self::Output<'a>> where Handler: $crate::error::ErrorHandler, Handler::Err: From<Err> {
                let $name = self;
                chain_parser!($($expr),+).parse(input, errs, ctx)
            }
        }
    }
}

impl_parser_tuple!(A, B; x = x.0, x.1);
impl_parser_tuple!(A, B, C; x = x.0, x.1, x.2);
impl_parser_tuple!(A, B, C, D; x = x.0, x.1, x.2, x.3);
impl_parser_tuple!(A, B, C, D, E; x = x.0, x.1, x.2, x.3, x.4);
impl_parser_tuple!(A, B, C, D, E, F; x = x.0, x.1, x.2, x.3, x.4, x.5);
impl_parser_tuple!(A, B, C, D, E, F, G; x = x.0, x.1, x.2, x.3, x.4, x.5, x.6);
impl_parser_tuple!(A, B, C, D, E, F, G, H; x = x.0, x.1, x.2, x.3, x.4, x.5, x.6, x.7);
impl_parser_tuple!(A, B, C, D, E, F, G, H, I; x = x.0, x.1, x.2, x.3, x.4, x.5, x.6, x.7, x.8);
impl_parser_tuple!(A, B, C, D, E, F, G, H, I, J; x = x.0, x.1, x.2, x.3, x.4, x.5, x.6, x.7, x.8, x.9);
impl_parser_tuple!(A, B, C, D, E, F, G, H, I, J, K; x = x.0, x.1, x.2, x.3, x.4, x.5, x.6, x.7, x.8, x.9, x.10);
impl_parser_tuple!(A, B, C, D, E, F, G, H, I, J, K, L; x = x.0, x.1, x.2, x.3, x.4, x.5, x.6, x.7, x.8, x.9, x.10, x.11);
