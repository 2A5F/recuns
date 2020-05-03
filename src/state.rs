use crate::*;
use std::error::*;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::sync::*;

// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct State<I, D = ()> {
    pub stop_when_err: bool,
    pub data: D,
    pub next: Box<dyn FnMut() -> Option<I>>,
    cancel: Option<Box<dyn Fn() -> bool>>,
    finish: bool,
    states: Vec<Box<dyn Recuns<Input = I, Data = D>>>,
    queue: Vec<Box<dyn FnMut(&mut Self)>>,
    errors: Vec<Arc<dyn Error>>,
    on_loop: Vec<Box<dyn FnMut(&mut Self)>>,
}
impl<I> State<I, ()> {
    #[inline]
    pub fn new_no_data(next: Box<dyn FnMut() -> Option<I>>, stop_when_err: bool) -> Self {
        Self {
            stop_when_err,
            data: (),
            next,
            cancel: None,
            finish: false,
            states: vec![],
            queue: vec![],
            errors: vec![],
            on_loop: vec![],
        }
    }
}
impl<I, D> State<I, D> {
    #[inline]
    pub fn new(next: Box<dyn FnMut() -> Option<I>>, stop_when_err: bool, data: D) -> Self {
        Self {
            stop_when_err,
            data,
            next,
            cancel: None,
            finish: false,
            states: vec![],
            queue: vec![],
            errors: vec![],
            on_loop: vec![],
        }
    }
    #[inline]
    pub fn set_cancel(&mut self, cancel: Box<dyn Fn() -> bool>) {
        self.cancel = Some(cancel);
    }
    #[inline]
    pub fn push(&mut self, rec: Box<dyn Recuns<Input = I, Data = D>>) {
        self.states.push(rec)
    }
    #[inline]
    fn pop(&mut self) -> Option<Box<dyn Recuns<Input = I, Data = D>>> {
        self.states.pop()
    }
    #[inline]
    pub fn on_loop(&mut self, f: Box<dyn FnMut(&mut Self)>) {
        self.on_loop.push(f)
    }
}
impl<I: Clone + 'static, D> State<I, D> {
    pub fn call(&mut self, input: I) -> Option<()> {
        let r = self.states.last_mut()?;
        let r = r.check(input.clone(), &mut self.data);

        match r {
            RecunsFlow::ReDo => self.queue.push(Box::new(move |this| {
                this.call(input.clone());
            })),
            RecunsFlow::End => {
                self.pop();
            }
            RecunsFlow::Call(f) => {
                self.push(f);
                self.queue.push(Box::new(move |this| {
                    this.call(input.clone());
                }))
            }
            RecunsFlow::CallNext(f) => self.push(f),
            RecunsFlow::Mov(f) => {
                self.pop();
                self.push(f);
                self.queue.push(Box::new(move |this| {
                    this.call(input.clone());
                }))
            }
            RecunsFlow::MovNext(f) => {
                self.pop();
                self.push(f);
            }
            RecunsFlow::Err(err) => {
                self.errors.push(err);
                if self.stop_when_err {
                    return None;
                }
            }
            RecunsFlow::None => (),
        }

        Some(())
    }
    pub fn do_loop(&mut self) -> Result<(), Vec<Arc<dyn Error>>> {
        loop {
            if let Some(ref cancel) = self.cancel {
                if cancel() {
                    return Ok(());
                }
            }
            if !self.queue.is_empty() {
                let mut q = self.queue.pop().unwrap();
                q(self);
                continue;
            }
            if self.finish {
                break;
            }
            let c = (self.next)();
            if c.is_none() {
                self.finish = true;
                continue;
            }
            let r = self.call(c.unwrap());
            if r.is_none() {
                break;
            }
        }
        if !self.errors.is_empty() {
            return Err(self.errors.clone());
        }
        Ok(())
    }
}
impl<I, D: Debug> Debug for State<I, D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("State")
            .field("stop_when_err", &self.stop_when_err)
            .field("data", &self.data)
            .field("data", &self.data)
            .finish()
    }
}
