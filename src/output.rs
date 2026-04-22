use std::{collections::HashMap, marker::PhantomData};

pub struct ToVec;

pub trait Collector<T> {
    type Container: Default;
    type Kind;

    fn consume(&self, container: &mut Self::Container, elem: T);
}

pub trait Insert {
    type Elem;
    fn insert(&mut self, elem: Self::Elem);
}

impl<T> Insert for Vec<T> {
    type Elem = T;

    fn insert(&mut self, elem: Self::Elem) {
        self.push(elem);
    }
}

impl<K, V> Insert for HashMap<K, V>
where
    K: std::hash::Hash + Eq,
{
    type Elem = (K, V);

    fn insert(&mut self, (k, v): Self::Elem) {
        HashMap::insert(self, k, v);
    }
}

pub struct Collect<C>(PhantomData<C>);

impl<C, T> Collector<T> for Collect<C>
where
    C: Default + Insert<Elem = T>,
{
    type Container = C;
    type Kind = Keep;

    fn consume(&self, container: &mut Self::Container, elem: T) {
        container.insert(elem);
    }
}

impl<C, T, D> DelimitedCollector<T, D> for Collect<C>
where
    C: Default + Insert<Elem = T>,
{
    type Container = C;
    type Kind = Keep;

    fn from(&self, elem: T) -> Self::Container {
        let mut empty = C::default();
        empty.insert(elem);
        empty
    }

    fn consume(&mut self, mut container: Self::Container, _delim: D, elem: T) -> Self::Container {
        container.insert(elem);
        container
    }
}

pub fn collect<C>() -> Collect<C>
where
    C: Default + Insert,
{
    Collect(PhantomData)
}

impl<T> Collector<T> for ToVec {
    type Container = Vec<T>;
    type Kind = Keep;

    fn consume(&self, container: &mut Self::Container, elem: T) {
        container.push(elem);
    }
}

impl<T> Collector<T> for Ignore {
    type Container = ();
    type Kind = Ignore;

    fn consume(&self, _container: &mut Self::Container, _elem: T) {}
}

pub trait DelimitedCollector<T, D> {
    type Container;
    type Kind;
    fn from(&self, elem: T) -> Self::Container;
    fn consume(&mut self, container: Self::Container, delim: D, elem: T) -> Self::Container;
}

impl<'a, T, D> DelimitedCollector<T, D> for ToVec {
    type Container = Vec<T>;
    type Kind = Keep;
    fn consume(&mut self, mut container: Self::Container, _delim: D, elem: T) -> Self::Container {
        container.push(elem);
        container
    }

    fn from(&self, elem: T) -> Self::Container {
        vec![elem]
    }
}

impl<'a, T, D> DelimitedCollector<T, D> for Ignore {
    type Container = ();
    type Kind = Ignore;
    fn consume(&mut self, _container: Self::Container, _delim: D, _elem: T) -> Self::Container {
        ()
    }

    fn from(&self, _elem: T) -> Self::Container {
        ()
    }
}

pub fn lfold<'a, Elem, Delim, F>(f: F) -> impl DelimitedCollector<Elem, Delim, Container = Elem>
where
    F: FnMut(Elem, Delim, Elem) -> Elem,
{
    struct LFold<F, Elem, Delim> {
        f: F,
        phantom: PhantomData<(Elem, Delim)>,
    }

    impl<'a, F, Elem, Delim> DelimitedCollector<Elem, Delim> for LFold<F, Elem, Delim>
    where
        F: FnMut(Elem, Delim, Elem) -> Elem,
    {
        type Container = Elem;
        type Kind = Keep;

        fn from(&self, elem: Elem) -> Self::Container {
            elem
        }

        fn consume(
            &mut self,
            container: Self::Container,
            delim: Delim,
            elem: Elem,
        ) -> Self::Container {
            (self.f)(container, delim, elem)
        }
    }

    LFold {
        f,
        phantom: PhantomData,
    }
}

pub trait OptionalOutput {
    type Output<T>;

    fn convert<V>(val: V) -> Self::Output<V>;
}

impl OptionalOutput for Keep {
    type Output<T> = T;

    fn convert<V>(val: V) -> Self::Output<V> {
        val
    }
}

impl OptionalOutput for Ignore {
    type Output<T> = ();

    fn convert<V>(_val: V) -> Self::Output<V> {
        ()
    }
}

pub trait Chain {
    type Output<A, B>;
    type NextKind;

    fn chain<A, B>(a: A, b: B) -> Self::Output<A, B>;
}

pub struct Ignore;
pub struct Keep;

pub struct ChainImpl<L, R> {
    phantom: PhantomData<(L, R)>,
}

impl Chain for ChainImpl<Keep, Keep> {
    type Output<A, B> = (A, B);
    type NextKind = Keep;

    fn chain<A, B>(a: A, b: B) -> Self::Output<A, B> {
        (a, b)
    }
}

impl Chain for ChainImpl<Keep, Ignore> {
    type Output<A, B> = A;
    type NextKind = Keep;

    fn chain<A, B>(a: A, _b: B) -> Self::Output<A, B> {
        a
    }
}

impl Chain for ChainImpl<Ignore, Keep> {
    type Output<A, B> = B;
    type NextKind = Keep;

    fn chain<A, B>(_a: A, b: B) -> Self::Output<A, B> {
        b
    }
}

impl Chain for ChainImpl<Ignore, Ignore> {
    type Output<A, B> = ();
    type NextKind = Ignore;

    fn chain<A, B>(_a: A, _b: B) -> Self::Output<A, B> {
        ()
    }
}
