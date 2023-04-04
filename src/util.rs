use either::Either;


pub trait UnwrapOrEmptyIter {
    type Iter: Iterator;

    fn unwrap_or_empty_iter(self) -> Self::Iter;
}

impl<T> UnwrapOrEmptyIter for Option<T>
where
    T: IntoIterator,
{
    type Iter = Either<T::IntoIter, std::iter::Empty<T::Item>>;

    fn unwrap_or_empty_iter(self) -> Self::Iter {
        self.map(|iter| Either::Left(iter.into_iter()))
            .unwrap_or_else(|| Either::Right(std::iter::empty()))
    }
}

macro_rules! fn_enum_is_variant {
    ($public:vis $name:ident, $variant:ident $(,)?) => {
        $public fn $name(&self) -> bool {
            matches!(self, Self::$variant { .. })
        }
    }
}

pub(crate) use fn_enum_is_variant;