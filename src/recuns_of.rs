use crate::*;

pub trait RecunsOfFn<Input, Data> {
    type OutPut: Recuns<Input = Input, Data = Data>;
    fn recuns(self) -> Self::OutPut;
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RecunsFnBox<F, Input, Data> {
    f: F,
    _i: std::marker::PhantomData<(Input, Data)>,
}
impl<F, Input, Data> RecunsFnBox<F, Input, Data>
where
    F: FnMut(Input, &mut Data) -> RecunsFlow<Input, Data>,
{
    #[inline]
    pub fn new(f: F) -> Self {
        Self {
            f,
            _i: std::marker::PhantomData,
        }
    }
}
impl<F, Input, Data> Recuns for RecunsFnBox<F, Input, Data>
where
    F: FnMut(Input, &mut Data) -> RecunsFlow<Input, Data>,
{
    type Input = Input;
    type Data = Data;

    #[inline]
    fn check(
        &mut self,
        input: Self::Input,
        data: &mut Self::Data,
    ) -> RecunsFlow<Self::Input, Self::Data> {
        (self.f)(input, data)
    }
}
impl<Input, F, Data> RecunsOfFn<Input, Data> for F
where
    F: FnMut(Input, &mut Data) -> RecunsFlow<Input, Data>,
{
    type OutPut = RecunsFnBox<F, Input, Data>;

    #[inline]
    fn recuns(self) -> Self::OutPut {
        RecunsFnBox::new(self)
    }
}
