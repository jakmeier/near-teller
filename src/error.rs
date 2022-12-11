#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub(crate) enum Error {
    NotEnoughHot,
    ForeignAccountNotAllowed,
}

impl Error {
    fn as_str(&self) -> &'static str {
        match self {
            Error::NotEnoughHot => "not enough hot tokens",
            Error::ForeignAccountNotAllowed => "must be called by contract account",
        }
    }

    /// Panic using the NEAR environment, no expensive string formatting going on.
    pub(crate) fn panic(&self) -> ! {
        near_sdk::env::panic_str(self.as_str());
    }
}
