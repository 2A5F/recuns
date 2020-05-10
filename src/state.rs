use crate::*;
use std::collections::VecDeque;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::sync::*;
use anyhow::Error;

// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct State<'a, 'b, I, D = ()> {
    pub stop_when_err: bool,

    pub data: D,

    pub states: Vec<Box<dyn 'a + Recuns<Input = I, Data = D>>>,
    pub queue: Vec<Box<dyn 'a + FnMut(&mut Self)>>,
    pub errors: &'b mut Vec<Arc<Error>>,
    // pub cancel: Option<Box<dyn 'a + Fn() -> bool>>,
    // pub finish: bool,
    // pub next: Box<dyn 'a + FnMut() -> Option<Result<I, Arc<Error>>>>,
    // pub on_loop: Option<Box<dyn FnMut(&mut Self)>>,
}
impl<'a, 'b, I> State<'a, 'b, I, ()> {
    #[inline]
    pub fn new_no_data(stop_when_err: bool, errors: &'b mut Vec<Arc<Error>>) -> Self {
        Self {
            stop_when_err,

            data: (),

            states: vec![],
            queue: vec![],
            errors,
            // on_loop: None,
            // next,
            // cancel: None,
            // finish: false,
        }
    }
}
impl<'a, 'b, I, D> State<'a, 'b, I, D> {
    #[inline]
    pub fn new(stop_when_err: bool, data: D, errors: &'b mut Vec<Arc<Error>>) -> Self {
        Self {
            stop_when_err,

            data,

            states: vec![],
            queue: vec![],
            errors,
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

fn call<'a, 'b, I: Clone + 'a, D>(s: &mut State<'a, 'b, I, D>, input: I, eof: bool) -> Option<()> {
    let r = s.states.last_mut()?;
    let r = r.check(input.clone(), &mut s.data, eof);

    #[inline(always)]
    fn redo<'a, 'b, I: Clone + 'a, D>(s: &mut State<'a, 'b, I, D>, input: I, eof: bool) {
        s.queue.push(Box::new(move |this| {
            call(this, input.clone(), eof);
        }));
    }

    match r {
        RecunsFlow::End => unsafe {
            s.pop();
        },
        RecunsFlow::EndReDo => unsafe {
            s.pop();
            redo(s, input, eof);
        },
        RecunsFlow::Call(f) => {
            s.push(f);
            redo(s, input, eof);
        }
        RecunsFlow::CallNext(f) => s.push(f),
        RecunsFlow::Mov(f) => unsafe {
            s.pop();
            s.push(f);
            redo(s, input, eof);
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
    { $s:ident ; $data:expr, $root:expr, $stop_when_err:expr, $next:expr ; $($b:block)? } => {
        let mut errors = vec![];
        let mut $s: State<'a, '_, I, D> = State::new($stop_when_err, $data, &mut errors);
        $s.push(Box::new($root));
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

            let c = $next(&mut $s.data);
            if c.is_none() {
                finish = true;
                let r = call(&mut $s, Default::default(), true);
                if r.is_none() {
                    break;
                }
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

            let r = call(&mut $s, c, false);
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
pub fn do_loop_cancel_on_loop<'a, I: Clone + Default + 'a, D>(
    data: D,
    root: impl Recuns<Data = D, Input = I> + 'a,
    stop_when_err: bool,
    mut next: impl FnMut(&mut D) -> Option<RecunsResult<I>>,
    mut cancel: impl FnMut() -> bool,
    mut on_loop: impl FnMut(&mut State<I, D>),
) -> RecunsResultErrs<()> {
    do_loop! {
        s ;
        data, root, stop_when_err, next ;
        {
            if cancel() {
                return Ok(());
            }
            on_loop(&mut s);
        }
    }
}
pub fn do_loop_on_loop<'a, I: Clone + Default + 'a, D>(
    data: D,
    root: impl Recuns<Data = D, Input = I> + 'a,
    stop_when_err: bool,
    mut next: impl FnMut(&mut D) -> Option<RecunsResult<I>>,
    mut on_loop: impl FnMut(&mut State<I, D>),
) -> RecunsResultErrs<()> {
    do_loop! {
        s ;
        data, root, stop_when_err, next ;
        {
            on_loop(&mut s);
        }
    }
}
pub fn do_loop_cancel<'a, I: Clone + Default + 'a, D>(
    data: D,
    root: impl Recuns<Data = D, Input = I> + 'a,
    stop_when_err: bool,
    mut next: impl FnMut(&mut D) -> Option<RecunsResult<I>>,
    mut cancel: impl FnMut() -> bool,
) -> RecunsResultErrs<()> {
    do_loop! {
        s ;
        data, root, stop_when_err, next ;
        {
            if cancel() {
                return Ok(());
            }
        }
    }
}
pub fn do_loop<'a, I: Clone + Default + 'a, D>(
    data: D,
    root: impl Recuns<Data = D, Input = I> + 'a,
    stop_when_err: bool,
    mut next: impl FnMut(&mut D) -> Option<RecunsResult<I>>,
) -> RecunsResultErrs<()> {
    // do_loop! {
    //     s ;
    //     data, root, stop_when_err, next ;
    // }
    let mut errors = vec![];
    let mut s: State<'_, '_, I, D> = State::new(stop_when_err, data, &mut errors);
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

        let c = next(&mut s.data);
        if c.is_none() {
            finish = true;
            let r = call(&mut s, Default::default(), true);
            if r.is_none() {
                break;
            }
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

        let r = call(&mut s, c, false);
        if r.is_none() {
            break;
        }
    }
    if !s.errors.is_empty() {
        return Err(s.errors.clone());
    }
    Ok(())
}

pub fn do_iter<'a, I: Clone + Default + 'a, D: 'a, U: 'a>(
    data: D,
    root: impl Recuns<Data = D, Input = I> + 'a,
    stop_when_err: bool,
    errors: &'a mut Vec<Arc<Error>>,
    mut next: impl 'a + FnMut(&mut D) -> Option<RecunsResult<I>>,
    mut yields: impl 'a + FnMut(&mut D) -> Option<VecDeque<U>>,
) -> impl 'a + Iterator<Item = U> {
    let mut s: State<'_, '_, I, D> = State::new(stop_when_err, data, errors);
    s.push(Box::new(root));
    let mut finish = false;
    let mut is_yield = false;
    let mut yield_val: VecDeque<U> = VecDeque::new();
    let i = DoLoopIter::new(move || -> Option<U> {
        loop {
            if !yield_val.is_empty() {
                let rv = yield_val.pop_front();
                if let Some(v) = rv {
                    return Some(v);
                }
            }
            if !is_yield {
                let r = yields(&mut s.data);
                if let Some(mut v) = r {
                    is_yield = true;
                    let rv = v.pop_front();
                    if !v.is_empty() {
                        yield_val = v;
                    }
                    if let Some(v) = rv {
                        return Some(v);
                    }
                }
            }
            is_yield = false;

            if !s.queue.is_empty() {
                let mut q = s.queue.pop().unwrap();
                q(&mut s);
                continue;
            }

            if finish {
                break;
            }

            let c = next(&mut s.data);
            if c.is_none() {
                finish = true;
                let r = call(&mut s, Default::default(), false);
                if r.is_none() {
                    break;
                }
                continue;
            }
            let c: RecunsResult<I> = c.unwrap();
            let c = match c {
                Ok(c) => c,
                Err(err) => {
                    s.errors.push(err);
                    return None;
                }
            };

            let r = call(&mut s, c, false);
            if r.is_none() {
                break;
            }
        }
        None
    });
    i
}

struct DoLoopIter<F> {
    f: F,
}
impl<F, U> DoLoopIter<F>
where
    F: FnMut() -> Option<U>,
{
    #[inline]
    pub fn new(f: F) -> Self {
        Self { f }
    }
}
impl<F, U> Iterator for DoLoopIter<F>
where
    F: FnMut() -> Option<U>,
{
    type Item = U;
    fn next(&mut self) -> Option<Self::Item> {
        (self.f)()
    }
}
impl<F> Debug for DoLoopIter<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("DoLoopIter").field("f", &"...").finish()
    }
}

//     pub fn do_loop(&mut self) -> Result<(), Vec<Arc<Error>>> {
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
//             let c: Result<I, Arc<Error>> = c.unwrap();
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
//             let c: Result<I, Arc<Error>> = c.unwrap();
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

#[macro_export]
macro_rules! try_ret {
    { $e:expr } => {
        if let Some(v) = $e {
            return v;
        }
    };
}
