pub mod recuns_of;
mod state;
pub use recuns_of::*;
pub use state::*;
use std::error::*;
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
pub type RecunsResult<T> = Result<T, Arc<dyn Error>>;
pub type RecunsResultErrs<T> = Result<T, Vec<Arc<dyn Error>>>;

pub enum RecunsFlow<I, D> {
    None,
    End,
    EndReDo,
    Call(Box<dyn Recuns<Input = I, Data = D>>),
    CallNext(Box<dyn Recuns<Input = I, Data = D>>),
    Mov(Box<dyn Recuns<Input = I, Data = D>>),
    MovNext(Box<dyn Recuns<Input = I, Data = D>>),
    Err(Arc<dyn Error>),
}
impl<I, D> RecunsFlow<I, D> {
    #[inline]
    pub fn call(r: impl Recuns<Input = I, Data = D> + 'static) -> Self {
        Self::Call(Box::new(r))
    }
    #[inline]
    pub fn call_next(r: impl Recuns<Input = I, Data = D> + 'static) -> Self {
        Self::CallNext(Box::new(r))
    }
    #[inline]
    pub fn mov(r: impl Recuns<Input = I, Data = D> + 'static) -> Self {
        Self::Mov(Box::new(r))
    }
    #[inline]
    pub fn mov_next(r: impl Recuns<Input = I, Data = D> + 'static) -> Self {
        Self::MovNext(Box::new(r))
    }
}
#[doc(hidden)]
pub trait RecunsEx<I, D> {
    fn rfcall(self) -> RecunsFlow<I, D>;
    fn rfcall_next(self) -> RecunsFlow<I, D>;
    fn rfmov(self) -> RecunsFlow<I, D>;
    fn rfmov_next(self) -> RecunsFlow<I, D>;
}
impl<R: 'static, I, D> RecunsEx<I, D> for R
where
    R: Recuns<Input = I, Data = D>,
{
    #[inline]
    fn rfcall(self) -> RecunsFlow<I, D> {
        RecunsFlow::call(self)
    }
    #[inline]
    fn rfcall_next(self) -> RecunsFlow<I, D> {
        RecunsFlow::call_next(self)
    }
    #[inline]
    fn rfmov(self) -> RecunsFlow<I, D> {
        RecunsFlow::mov(self)
    }
    #[inline]
    fn rfmov_next(self) -> RecunsFlow<I, D> {
        RecunsFlow::mov_next(self)
    }
}
#[doc(hidden)]
pub trait RecunsFnEx<I, D> {
    fn rfcall(self) -> RecunsFlow<I, D>;
    fn rfcall_next(self) -> RecunsFlow<I, D>;
    fn rfmov(self) -> RecunsFlow<I, D>;
    fn rfmov_next(self) -> RecunsFlow<I, D>;
}
impl<F: 'static, I: 'static, D: 'static> RecunsFnEx<I, D> for F
where
    F: FnMut(I, &mut D, bool) -> RecunsFlow<I, D>,
{
    #[inline]
    fn rfcall(self) -> RecunsFlow<I, D> {
        RecunsFlow::call(self.recuns())
    }
    #[inline]
    fn rfcall_next(self) -> RecunsFlow<I, D> {
        RecunsFlow::call_next(self.recuns())
    }
    #[inline]
    fn rfmov(self) -> RecunsFlow<I, D> {
        RecunsFlow::mov(self.recuns())
    }
    #[inline]
    fn rfmov_next(self) -> RecunsFlow<I, D> {
        RecunsFlow::mov_next(self.recuns())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
