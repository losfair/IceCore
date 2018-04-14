use std::error::Error;

macro_rules! impl_debug_display {
    ($target:ident) => {
        impl ::std::fmt::Display for $target {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                <Self as ::std::fmt::Debug>::fmt(self, f)
            }
        }
    }
}

#[derive(Debug)]
pub enum Io {
    Generic,
    Custom(String)
}

pub type IoResult<T> = Result<T, Io>;

impl Error for Io {
    fn description(&self) -> &str {
        "I/O Error"
    }
}

impl_debug_display!(Io);
