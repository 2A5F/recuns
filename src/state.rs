use crate::*;
use std::error::*;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::sync::*;

// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct State<'a, I, D = ()> {
    pub stop_when_err: bool,

    pub data: D,

    pub states: Vec<Box<dyn 'a + Recuns<Input = I, Data = D>>>,
    pub queue: Vec<Box<dyn 'a + FnMut(&mut Self)>>,
    pub errors: Vec<Arc<dyn Error>>,
    // pub cancel: Option<Box<dyn 'a + Fn() -> bool>>,
    // pub finish: bool,
    // pub next: Box<dyn 'a + FnMut() -> Option<Result<I, Arc<dyn Error>>>>,
    // pub on_loop: Option<Box<dyn FnMut(&mut Self)>>,
}
impl<'a, I> State<'a, I, ()> {
    #[inline]
    pub fn new_no_data(stop_when_err: bool) -> Self {
        Self {
            stop_when_err,

            data: (),

            states: vec![],
            queue: vec![],
            errors: vec![],
            // on_loop: None,
            // next,
            // cancel: None,
            // finish: false,
        }
    }
}
impl<'a, I, D> State<'a, I, D> {
    #[inline]
    pub fn new(stop_when_err: bool, data: D) -> Self {
        Self {
            stop_when_err,

            data,

            states: vec![],
            queue: vec![],
            errors: vec![],
            // on_loop: None,
            // next,
            // cancel: None,
            // finish: false,
        }
    }
    #[inline]
    pub fn push(&mut self, rec: Box<dyn 'a + Recuns<Input = I, Data = D>>) {
        self.states.push(rec)
    }
    #[inline]
    pub unsafe fn pop(&mut self) -> Option<Box<dyn 'a + Recuns<Input = I, Data = D>>> {
        self.states.pop()
    }
}
// impl<'a, I: Clone + 'a, D> State<'a, I, D> {
//     fn call(&mut self, input: I) -> Option<()> {
//         let r = self.states.last_mut()?;
//         let r = r.check(input.clone(), &mut self.data);

//         match r {
//             RecunsFlow::ReDo => self.queue.push(Box::new(move |this| {
//                 this.call(input.clone());
//             })),
//             RecunsFlow::End => {
//                 self.pop();
//             }
//             RecunsFlow::Call(f) => {
//                 self.push(f);
//                 self.queue.push(Box::new(move |this| {
//                     this.call(input.clone());
//                 }))
//             }
//             RecunsFlow::CallNext(f) => self.push(f),
//             RecunsFlow::Mov(f) => {
//                 self.pop();
//                 self.push(f);
//                 self.queue.push(Box::new(move |this| {
//                     this.call(input.clone());
//                 }))
//             }
//             RecunsFlow::MovNext(f) => {
//                 self.pop();
//                 self.push(f);
//             }
//             RecunsFlow::Err(err) => {
//                 self.errors.push(err);
//                 if self.stop_when_err {
//                     return None;
//                 }
//             }
//             RecunsFlow::None => (),
//         }

//         Some(())
//     }

fn call<'a, I: Clone + 'a, D>(s: &mut State<'a, I, D>, input: I) -> Option<()> {
    let r = s.states.last_mut()?;
    let r = r.check(input.clone(), &mut s.data);

    match r {
        RecunsFlow::ReDo => s.queue.push(Box::new(move |this| {
            call(this, input.clone());
        })),
        RecunsFlow::End => unsafe {
            s.pop();
        },
        RecunsFlow::Call(f) => {
            s.push(f);
            s.queue.push(Box::new(move |this| {
                call(this, input.clone());
            }))
        }
        RecunsFlow::CallNext(f) => s.push(f),
        RecunsFlow::Mov(f) => unsafe {
            s.pop();
            s.push(f);
            s.queue.push(Box::new(move |this| {
                call(this, input.clone());
            }))
        },
        RecunsFlow::MovNext(f) => unsafe {
            s.pop();
            s.push(f);
        },
        RecunsFlow::Err(err) => {
            s.errors.push(err);
            if s.stop_when_err {
                return None;
            }
        }
        RecunsFlow::None => (),
    }

    Some(())
}

macro_rules! do_loop {
    { $s:ident ; $data:expr, $stop_when_err:expr, $next:expr ; $($b:block)? } => {
        let mut $s: State<'a, I, D> = State::new($stop_when_err, $data);
        let mut finish = false;
        loop {
            $($b;)?

            if !$s.queue.is_empty() {
                let mut q = $s.queue.pop().unwrap();
                q(&mut $s);
                continue;
            }

            if finish {
                break;
            }

            let c = $next();
            if c.is_none() {
                finish = true;
                continue;
            }
            let c: RecunsResult<I> = c.unwrap();
            let c = match c {
                Ok(c) => c,
                Err(err) => {
                    $s.errors.push(err);
                    return Err($s.errors.clone());
                }
            };

            let r = call(&mut $s, c);
            if r.is_none() {
                break;
            }
        }
        if !$s.errors.is_empty() {
            return Err($s.errors.clone());
        }
        Ok(())
    };
}
pub fn do_loop_cancel_on_loop<'a, I: Clone + 'a, D>(
    data: D,
    stop_when_err: bool,
    mut next: impl FnMut() -> Option<RecunsResult<I>>,
    mut cancel: impl FnMut() -> bool,
    mut on_loop: impl FnMut(&mut State<I, D>),
) -> RecunsResultErrs<()> {
    do_loop! {
        s ;
        data, stop_when_err, next ;
        {
            if cancel() {
                return Ok(());
            }
            on_loop(&mut s);
        }
    }
}
pub fn do_loop_on_loop<'a, I: Clone + 'a, D>(
    data: D,
    stop_when_err: bool,
    mut next: impl FnMut() -> Option<RecunsResult<I>>,
    mut on_loop: impl FnMut(&mut State<I, D>),
) -> RecunsResultErrs<()> {
    do_loop! {
        s ;
        data, stop_when_err, next ;
        {
            on_loop(&mut s);
        }
    }
}
pub fn do_loop_cancel<'a, I: Clone + 'a, D>(
    data: D,
    stop_when_err: bool,
    mut next: impl FnMut() -> Option<RecunsResult<I>>,
    mut cancel: impl FnMut() -> bool,
) -> RecunsResultErrs<()> {
    do_loop! {
        s ;
        data, stop_when_err, next ;
        {
            if cancel() {
                return Ok(());
            }
        }
    }
}
pub fn do_loop<'a, I: Clone + 'a, D>(
    data: D,
    root: impl Recuns<Data = D, Input = I> + 'static,
    stop_when_err: bool,
    mut next: impl FnMut() -> Option<RecunsResult<I>>,
) -> RecunsResultErrs<()> {
    // do_loop! {
    //     s ;
    //     data, stop_when_err, next ;
    // }
    let mut s: State<'a, I, D> = State::new(stop_when_err, data);
    s.push(Box::new(root));
    let mut finish = false;
    loop {
        if !s.queue.is_empty() {
            let mut q = s.queue.pop().unwrap();
            q(&mut s);
            continue;
        }

        if finish {
            break;
        }

        let c = next();
        if c.is_none() {
            finish = true;
            continue;
        }
        let c: RecunsResult<I> = c.unwrap();
        let c = match c {
            Ok(c) => c,
            Err(err) => {
                s.errors.push(err);
                return Err(s.errors.clone());
            }
        };

        let r = call(&mut s, c);
        if r.is_none() {
            break;
        }
    }
    if !s.errors.is_empty() {
        return Err(s.errors.clone());
    }
    Ok(())
}

//     pub fn do_loop(&mut self) -> Result<(), Vec<Arc<dyn Error>>> {
//         let this = unsafe { &mut *(self as *mut _) };
//         loop {
//             if let Some(ref cancel) = self.cancel {
//                 if cancel() {
//                     return Ok(());
//                 }
//             }
//             if let Some(ref mut v) = self.on_loop {
//                 v(this);
//             }
//             if !self.queue.is_empty() {
//                 let mut q = self.queue.pop().unwrap();
//                 q(self);
//                 continue;
//             }
//             if self.finish {
//                 break;
//             }
//             let c = (self.next)();
//             if c.is_none() {
//                 self.finish = true;
//                 continue;
//             }
//             let c: Result<I, Arc<dyn Error>> = c.unwrap();
//             let c = match c {
//                 Ok(c) => c,
//                 Err(err) => {
//                     self.errors.push(err);
//                     return Err(self.errors.clone());
//                 }
//             };
//             let r = self.call(c);
//             if r.is_none() {
//                 break;
//             }
//         }
//         if !self.errors.is_empty() {
//             return Err(self.errors.clone());
//         }
//         Ok(())
//     }
//     pub fn iter_loop<F, U>(&mut self, f: F) -> StateIter<'_, 'a, F, I, D>
//     where
//         F: FnMut(&mut Self) -> Option<U>,
//     {
//         StateIter::new(f, self)
//     }
// }
// impl<'a, I, D: Debug> Debug for State<'a, I, D> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         f.debug_struct("State")
//             .field("stop_when_err", &self.stop_when_err)
//             .field("data", &self.data)
//             .field("data", &self.data)
//             .finish()
//     }
// }

// #[derive(Debug)]
// pub struct StateIter<'a, 'b, F, I, D = ()> {
//     f: F,
//     s: &'a mut State<'b, I, D>,
//     yield_: bool,
// }
// impl<'a, 'b, F, I, D, U> StateIter<'a, 'b, F, I, D>
// where
//     F: FnMut(&mut State<'b, I, D>) -> Option<U>,
// {
//     pub fn new(f: F, s: &'a mut State<'b, I, D>) -> Self {
//         Self {
//             f,
//             s,
//             yield_: false,
//         }
//     }
// }
// impl<'a, 'b, F, I: Clone + 'static, D, U> Iterator for StateIter<'a, 'b, F, I, D>
// where
//     F: FnMut(&mut State<I, D>) -> Option<U>,
// {
//     type Item = U;

//     fn next(&mut self) -> Option<Self::Item> {
//         let this = unsafe { &mut *(self.s as *mut _) };
//         loop {
//             if let Some(ref cancel) = self.s.cancel {
//                 if cancel() {
//                     return None;
//                 }
//             }
//             if !self.yield_ {
//                 let r: Option<U> = (self.f)(self.s);
//                 if let Some(v) = r {
//                     self.yield_ = true;
//                     return Some(v);
//                 }
//             }
//             self.yield_ = false;
//             if let Some(ref mut v) = self.s.on_loop {
//                 v(this);
//             }
//             if !self.s.queue.is_empty() {
//                 let mut q = self.s.queue.pop().unwrap();
//                 q(self.s);
//                 continue;
//             }
//             if self.s.finish {
//                 break;
//             }
//             let c = (self.s.next)();
//             if c.is_none() {
//                 self.s.finish = true;
//                 continue;
//             }
//             let c: Result<I, Arc<dyn Error>> = c.unwrap();
//             let c = match c {
//                 Ok(c) => c,
//                 Err(err) => {
//                     self.s.errors.push(err);
//                     return None;
//                 }
//             };
//             let r = self.s.call(c);
//             if r.is_none() {
//                 break;
//             }
//         }
//         None
//     }
// }
