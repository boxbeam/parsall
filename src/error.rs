use std::{
    cell::UnsafeCell,
    error::Error,
    fmt::{Debug, Display},
    marker::PhantomData,
    ops::{Range, RangeInclusive},
};

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    /// Thrown when a literal substring is expected.
    ExpectedLiteral(&'static str, &'static str),
    /// Thrown when a literal character is expected.
    ExpectedChar(char),
    /// Thrown when an unspecified parsing operation associated with a named parser fails.
    ExpectedToken(&'static str),
    /// Thrown when one of a set of symbols is expected
    ExpectedSymbol(&'static [char]),
    /// Thrown when one of a set of symbols is expected
    ExpectedRange(RangeInclusive<char>),
    /// Thrown when parsing succeeds, but there are more tokens left in the input which were not consumed.
    UnexpectedToken,
}

impl Error for ParseError {}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::ExpectedLiteral(lit, parser_name) => write!(
                f,
                "Expected '{lit}' while parsing {parser_name}",
                lit = lit.replace('\n', "<newline>").replace('\t', "<tab>"),
                parser_name = parser_name.replace('_', " ")
            ),
            ParseError::ExpectedToken(token) => write!(f, "Expected {}", token.replace('_', " ")),
            ParseError::UnexpectedToken => write!(f, "Unexpected token"),
            ParseError::ExpectedSymbol(items) => write!(f, "Expected one of {items:?}"),
            ParseError::ExpectedChar(c) => write!(f, "Expected '{c}'"),
            ParseError::ExpectedRange(range_inclusive) => {
                write!(f, "Expected symbol in {range_inclusive:?}")
            }
        }
    }
}

pub trait ErrorHandler: Clone {
    type Err: From<ParseError>;
    fn error(&self, err: impl Into<Self::Err>, loc: Range<usize>);
}

#[derive(Default)]
pub struct DebugErrorHandler<E>(PhantomData<E>);

impl<E> Clone for DebugErrorHandler<E> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<E> ErrorHandler for DebugErrorHandler<E>
where
    E: Debug + From<ParseError>,
{
    type Err = E;
    fn error(&self, err: impl Into<E>, loc: Range<usize>) {
        let err = err.into();
        eprintln!("Error at {loc:?}: {err:?}");
    }
}

#[derive(Debug)]
pub struct ErrorLocation<E>(pub E, pub Range<usize>);

pub struct ErrorCell<E> {
    inner: UnsafeCell<Option<ErrorLocation<E>>>,
}

impl<E> Default for ErrorCell<E> {
    fn default() -> Self {
        Self { inner: None.into() }
    }
}

impl<E> ErrorHandler for &ErrorCell<E>
where
    E: From<ParseError>,
{
    type Err = E;
    fn error(&self, err: impl Into<E>, loc: Range<usize>) {
        unsafe {
            let inner = self.inner.get();
            // if (*inner).as_ref().is_none_or() {

            // }
            (*inner).replace(ErrorLocation(err.into(), loc));
        }
    }
}

impl<E> ErrorCell<E> {
    pub fn into_inner(self) -> Option<ErrorLocation<E>> {
        self.inner.into_inner()
    }
}
