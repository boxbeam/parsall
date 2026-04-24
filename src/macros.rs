pub use {
    crate::err_type, crate::keep_type, crate::map_or_try_map, crate::names_pattern,
    crate::not_drop, crate::p, crate::parser_fn, crate::parser_fns, crate::parsers_choice,
    crate::pmatch, crate::ret_type,
};

#[macro_export]
macro_rules! parsers_choice {
    ($val:expr) => {
        $val
    };
    ($val:expr $(, $($rest:expr),+ )?) => { $val.or($crate::parsers_choice!( $( $($rest),+ )? )) };
}

#[macro_export]
macro_rules! p {
    ($val:expr) => {
        $val
    };
    ($val:expr $(, $($rest:expr),+ )?) => { $val.then($crate::p!( $( $($rest),+ )? )) };
}

#[macro_export]
macro_rules! not_drop {
    ($parser:expr) => {
        $parser.drop()
    };
    ($parser:expr, $name:ident) => {
        $parser
    };
}

#[macro_export]
macro_rules! names_pattern {
    ($name:ident $(,)?) => {
        $name
    };
    ($a:ident , $($rest:ident),+ $(,)?) => {
        ($a, $crate::names_pattern!($($rest),+) , )
    };
    (,) => {};
    () => {
        _
    };
}

#[macro_export]
macro_rules! ret_type {
    ($ty:ty) => {
        $ty
    };
    () => {
        ()
    };
}

#[macro_export]
macro_rules! err_type {
    ($ty:ty) => {
        $ty
    };
    () => {
        $crate::error::ParseError
    };
}

#[macro_export]
macro_rules! keep_type {
    ($ty:ty) => {
        $crate::output::Keep
    };
    () => {
        $crate::output::Ignore
    };
}

#[macro_export]
macro_rules! map_or_try_map {
    ($err_typ:ty | $parser:expr ; $names_pat:pat => $block:block) => {
        $parser.try_map::<_, _, $err_typ>(|$names_pat| $block)
    };
    (| $parser:expr ; $names_pat:pat => $block:block) => {
        $parser.map(|$names_pat| $block)
    };
}

#[macro_export]
macro_rules! parser_fn {
    ($vis:vis $name:ident ($( $(@ $match_name:ident =)? $parser:expr),*) -> $ret:ty $(, $err_ret:ty)? $block:block) => {
        #[allow(non_camel_case_types)]
        struct $name;

        impl<E> Parser<E, ()> for $name where E: From<$crate::err_type!($($err_ret)?)> {
            type Output<'a> = $ret;
            type Kind = Keep;

            fn parse<'a>(
                &mut self,
                input: $crate::Input<'a>,
                errs: impl $crate::error::ErrorHandler<E>,
                ctx: &mut (),
            ) -> ParserResult<$ret> {
                let (__len, val) = Parser::<$crate::err_type!($($err_ret)?), _>::parse(&mut $crate::map_or_try_map!(
                    $($err_ret)? |
                    $crate::p!($($crate::not_drop!($parser $(, $match_name)?)),*) ;
                    $crate::names_pattern!($($($match_name ,)?)*) => $block
                ), input, |e, r| errs.error(e, r), ctx)?;
                Some((__len, val))
            }
        }
    };
    ($vis:vis $name:ident ($( $parser:expr),*) $(-> $ret:ty $(, $err_ret:ty)?)?) => {
        #[allow(non_camel_case_types)]
        $vis struct $name;

        impl<E> Parser<E, ()> for $name where E: From<$crate::err_type!($($($err_ret)?)?)> {
            type Output<'a> = $crate::ret_type!($($ret)?);
            type Kind = $crate::keep_type!($($ret)?);

            fn parse<'a>(
                &mut self,
                input: $crate::Input<'a>,
                errs: impl $crate::error::ErrorHandler<E>,
                ctx: &mut (),
            ) -> ParserResult<$crate::ret_type!($($ret)?)> {
                let (__len, val) = Parser::<$crate::err_type!($($($err_ret)?)?), _>::parse(&mut $crate::p!($($parser),*), input, move |e, r| errs.error(e, r), ctx)?;
                Some((__len, val))
            }
        }
    }
}

#[macro_export]
macro_rules! parser_fns {
    ($($vis:vis $name:ident ($($tt:tt)*) $(-> $ret:ty $(, $err_ret:ty)?)? $($block:block)? ;)* ) => { $( $crate::parser_fn!( $vis $name ( $($tt)* ) $(-> $ret $(, $err_ret)?)? $($block)? ); )+ };
}

#[macro_export]
macro_rules! pmatch {
    ( $( $($(@ $match_name:ident =)? $parser:expr),* => $val:expr ),+ $(,)?) => {
        $crate::parsers_choice!( $( $crate::p!( $($crate::not_drop!($parser $(, $match_name)?)),* ).map(|$crate::names_pattern!( $($($match_name,)?)? )| { $val } ) ),* )
    };
}
