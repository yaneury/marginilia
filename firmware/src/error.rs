#[derive(Debug)]
pub enum Error<E> {
    Spi(E),
}

impl<E> From<core::convert::Infallible> for Error<E> {
    fn from(e: core::convert::Infallible) -> Self {
        match e {}
    }
}

impl<E: core::fmt::Display> core::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Spi(e) => write!(f, "SPI error: {e}"),
        }
    }
}
