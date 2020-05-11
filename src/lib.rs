pub mod recuns_of;
mod state;
use anyhow::Error;
pub use recuns_of::*;
pub use state::*;
use std::sync::*;

#[cfg(test)]
mod test_json;

pub trait Recuns {
    type Input;
    type Data;

    fn check(
        &mut self,
        input: Self::Input,
        data: &mut Self::Data,
        eof: bool,
    ) -> RecunsFlow<Self::Input, Self::Data>;
}
pub type RecunsResult<T> = Result<T, Arc<Error>>;
pub type RecunsResultErrs<T> = Result<T, Vec<Arc<Error>>>;

pub enum RecunsFlow<I, D> {
    None,
    End,
    EndReDo,
    Call(Box<dyn Recuns<Input = I, Data = D>>, &'static str),
    CallNext(Box<dyn Recuns<Input = I, Data = D>>, &'static str),
    Mov(Box<dyn Recuns<Input = I, Data = D>>, &'static str),
    MovNext(Box<dyn Recuns<Input = I, Data = D>>, &'static str),
    Err(Arc<Error>),
}
impl<I, D> std::fmt::Debug for RecunsFlow<I, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::End => write!(f, "End"),
            Self::EndReDo => write!(f, "EndReDo"),
            Self::Call(_, name) => write!(f, "Call({})", name),
            Self::CallNext(_, name) => write!(f, "CallNext({})", name),
            Self::Mov(_, name) => write!(f, "Mov({})", name),
            Self::MovNext(_, name) => write!(f, "MovNext({})", name),
            Self::Err(err) => write!(f, "Err({:?})", err),
        }
    }
}
impl<I, D> From<Arc<Error>> for RecunsFlow<I, D> {
    #[inline]
    fn from(e: Arc<Error>) -> Self {
        Self::Err(e)
    }
}
impl<I, D> From<Error> for RecunsFlow<I, D> {
    #[inline]
    fn from(e: Error) -> Self {
        Self::Err(Arc::new(e))
    }
}
impl<I, D> RecunsFlow<I, D> {
    #[inline]
    pub fn call(name: &'static str, r: impl Recuns<Input = I, Data = D> + 'static) -> Self {
        Self::Call(Box::new(r), name)
    }
    #[inline]
    pub fn call_next(name: &'static str, r: impl Recuns<Input = I, Data = D> + 'static) -> Self {
        Self::CallNext(Box::new(r), name)
    }
    #[inline]
    pub fn mov(name: &'static str, r: impl Recuns<Input = I, Data = D> + 'static) -> Self {
        Self::Mov(Box::new(r), name)
    }
    #[inline]
    pub fn mov_next(name: &'static str, r: impl Recuns<Input = I, Data = D> + 'static) -> Self {
        Self::MovNext(Box::new(r), name)
    }
}
#[doc(hidden)]
pub trait RecunsEx<I, D> {
    fn rfcall(self, name: &'static str) -> RecunsFlow<I, D>;
    fn rfcall_next(self, name: &'static str) -> RecunsFlow<I, D>;
    fn rfmov(self, name: &'static str) -> RecunsFlow<I, D>;
    fn rfmov_next(self, name: &'static str) -> RecunsFlow<I, D>;
}
impl<R: 'static, I, D> RecunsEx<I, D> for R
where
    R: Recuns<Input = I, Data = D>,
{
    #[inline]
    fn rfcall(self, name: &'static str) -> RecunsFlow<I, D> {
        RecunsFlow::call(name, self)
    }
    #[inline]
    fn rfcall_next(self, name: &'static str) -> RecunsFlow<I, D> {
        RecunsFlow::call_next(name, self)
    }
    #[inline]
    fn rfmov(self, name: &'static str) -> RecunsFlow<I, D> {
        RecunsFlow::mov(name, self)
    }
    #[inline]
    fn rfmov_next(self, name: &'static str) -> RecunsFlow<I, D> {
        RecunsFlow::mov_next(name, self)
    }
}
#[doc(hidden)]
pub trait RecunsFnEx<I, D> {
    fn rfcall(self, name: &'static str) -> RecunsFlow<I, D>;
    fn rfcall_next(self, name: &'static str) -> RecunsFlow<I, D>;
    fn rfmov(self, name: &'static str) -> RecunsFlow<I, D>;
    fn rfmov_next(self, name: &'static str) -> RecunsFlow<I, D>;
}
impl<F: 'static, I: 'static, D: 'static> RecunsFnEx<I, D> for F
where
    F: FnMut(I, &mut D, bool) -> RecunsFlow<I, D>,
{
    #[inline]
    fn rfcall(self, name: &'static str) -> RecunsFlow<I, D> {
        RecunsFlow::call(name, self.recuns())
    }
    #[inline]
    fn rfcall_next(self, name: &'static str) -> RecunsFlow<I, D> {
        RecunsFlow::call_next(name, self.recuns())
    }
    #[inline]
    fn rfmov(self, name: &'static str) -> RecunsFlow<I, D> {
        RecunsFlow::mov(name, self.recuns())
    }
    #[inline]
    fn rfmov_next(self, name: &'static str) -> RecunsFlow<I, D> {
        RecunsFlow::mov_next(name, self.recuns())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
