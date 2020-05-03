pub mod recuns_of;
mod state;
pub use recuns_of::*;
pub use state::*;
use std::error::*;
use std::sync::*;

pub trait Recuns {
    type Input;
    type Data;

    fn check(&mut self, input: Self::Input, data: &mut Self::Data) -> RecunsFlow<Self::Input, Self::Data>;
}

pub enum RecunsFlow<I, D> {
    None,
    End,
    ReDo,
    Call(Box<dyn Recuns<Input = I, Data = D>>),
    CallNext(Box<dyn Recuns<Input = I, Data = D>>),
    Mov(Box<dyn Recuns<Input = I, Data = D>>),
    MovNext(Box<dyn Recuns<Input = I, Data = D>>),
    Err(Arc<dyn Error>),
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
