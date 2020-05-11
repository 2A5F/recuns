use crate::*;
use anyhow::Error;
use std::collections::VecDeque;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::sync::*;

pub struct State<'a, 'b, I, D = ()> {
    pub stop_when_err: bool,

    pub data: D,

    pub states: Vec<Box<dyn 'a + Recuns<Input = I, Data = D>>>,
    pub queue: Vec<Box<dyn 'a + FnMut(&mut Self)>>,
    pub errors: &'b mut Vec<Arc<Error>>,
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

#[inline]
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
        RecunsFlow::Call(f, _) => {
            s.push(f);
            redo(s, input, eof);
        }
        RecunsFlow::CallNext(f, _) => s.push(f),
        RecunsFlow::Mov(f, _) => unsafe {
            s.pop();
            s.push(f);
            redo(s, input, eof);
        },
        RecunsFlow::MovNext(f, _) => unsafe {
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
        Ok(Some($s.data))
    };
}

#[inline]
pub fn do_loop_cancel_on_loop<'a, I: Clone + Default + 'a, D>(
    data: D,
    root: impl Recuns<Data = D, Input = I> + 'a,
    stop_when_err: bool,
    mut next: impl FnMut(&mut D) -> Option<RecunsResult<I>>,
    mut cancel: impl FnMut() -> bool,
    mut on_loop: impl FnMut(&mut State<I, D>),
) -> RecunsResultErrs<Option<D>> {
    do_loop! {
        s ;
        data, root, stop_when_err, next ;
        {
            if cancel() {
                return Ok(None);
            }
            on_loop(&mut s);
        }
    }
}

#[inline]
pub fn do_loop_on_loop<'a, I: Clone + Default + 'a, D>(
    data: D,
    root: impl Recuns<Data = D, Input = I> + 'a,
    stop_when_err: bool,
    mut next: impl FnMut(&mut D) -> Option<RecunsResult<I>>,
    mut on_loop: impl FnMut(&mut State<I, D>),
) -> RecunsResultErrs<Option<D>> {
    do_loop! {
        s ;
        data, root, stop_when_err, next ;
        {
            on_loop(&mut s);
        }
    }
}

#[inline]
pub fn do_loop_cancel<'a, I: Clone + Default + 'a, D>(
    data: D,
    root: impl Recuns<Data = D, Input = I> + 'a,
    stop_when_err: bool,
    mut next: impl FnMut(&mut D) -> Option<RecunsResult<I>>,
    mut cancel: impl FnMut() -> bool,
) -> RecunsResultErrs<Option<D>> {
    do_loop! {
        s ;
        data, root, stop_when_err, next ;
        {
            if cancel() {
                return Ok(None);
            }
        }
    }
}

#[inline]
pub fn do_loop<'a, I: Clone + Default + 'a, D>(
    data: D,
    root: impl Recuns<Data = D, Input = I> + 'a,
    stop_when_err: bool,
    mut next: impl FnMut(&mut D) -> Option<RecunsResult<I>>,
) -> RecunsResultErrs<Option<D>> {
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
    Ok(Some(s.data))
}

macro_rules! do_iter { //$($b;)?
    { $s:ident ; $data:expr, $root:expr, $stop_when_err:expr, $next:expr, $errors:expr, $yields:expr ; $($b:block)? } => {
        let mut $s: State<'_, '_, I, D> = State::new($stop_when_err, $data, $errors);
        $s.push(Box::new($root));
        let mut finish = false;
        let mut is_yield = false;
        let mut yield_val: VecDeque<U> = VecDeque::new();
        let i = DoLoopIter::new(move || -> Option<U> {
            loop {
                $($b;)?

                if !yield_val.is_empty() {
                    let rv = yield_val.pop_front();
                    if let Some(v) = rv {
                        return Some(v);
                    }
                }
                if !is_yield {
                    let r = $yields(&mut $s.data);
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
                    let r = call(&mut $s, Default::default(), false);
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
                        return None;
                    }
                };

                let r = call(&mut $s, c, false);
                if r.is_none() {
                    break;
                }
            }
            None
        });
        i
    };
}

#[inline]
pub fn do_iter_cancel_on_loop<'a, I: Clone + Default + 'a, D: 'a, U: 'a>(
    data: D,
    root: impl Recuns<Data = D, Input = I> + 'a,
    stop_when_err: bool,
    errors: &'a mut Vec<Arc<Error>>,
    mut next: impl 'a + FnMut(&mut D) -> Option<RecunsResult<I>>,
    mut yields: impl 'a + FnMut(&mut D) -> Option<VecDeque<U>>,
    mut cancel: impl 'a + FnMut() -> bool,
    mut on_loop: impl 'a + FnMut(&mut State<I, D>),
) -> impl 'a + Iterator<Item = U> {
    do_iter! {
        s ;
        data, root, stop_when_err, next, errors, yields ;
        {
            if cancel() {
                return None;
            }
            on_loop(&mut s);
        }
    }
}

#[inline]
pub fn do_iter_on_loop<'a, I: Clone + Default + 'a, D: 'a, U: 'a>(
    data: D,
    root: impl Recuns<Data = D, Input = I> + 'a,
    stop_when_err: bool,
    errors: &'a mut Vec<Arc<Error>>,
    mut next: impl 'a + FnMut(&mut D) -> Option<RecunsResult<I>>,
    mut yields: impl 'a + FnMut(&mut D) -> Option<VecDeque<U>>,
    mut on_loop: impl 'a + FnMut(&mut State<I, D>),
) -> impl 'a + Iterator<Item = U> {
    do_iter! {
        s ;
        data, root, stop_when_err, next, errors, yields ;
        {
            on_loop(&mut s);
        }
    }
}

#[inline]
pub fn do_iter_cancel<'a, I: Clone + Default + 'a, D: 'a, U: 'a>(
    data: D,
    root: impl Recuns<Data = D, Input = I> + 'a,
    stop_when_err: bool,
    errors: &'a mut Vec<Arc<Error>>,
    mut next: impl 'a + FnMut(&mut D) -> Option<RecunsResult<I>>,
    mut yields: impl 'a + FnMut(&mut D) -> Option<VecDeque<U>>,
    mut cancel: impl 'a + FnMut() -> bool,
) -> impl 'a + Iterator<Item = U> {
    do_iter! {
        s ;
        data, root, stop_when_err, next, errors, yields ;
        {
            if cancel() {
                return None;
            }
        }
    }
}

#[inline]
pub fn do_iter<'a, I: Clone + Default + 'a, D: 'a, U: 'a>(
    data: D,
    root: impl Recuns<Data = D, Input = I> + 'a,
    stop_when_err: bool,
    errors: &'a mut Vec<Arc<Error>>,
    mut next: impl 'a + FnMut(&mut D) -> Option<RecunsResult<I>>,
    mut yields: impl 'a + FnMut(&mut D) -> Option<VecDeque<U>>,
) -> impl 'a + Iterator<Item = U> {
    do_iter! {
        s ;
        data, root, stop_when_err, next, errors, yields ;
    }
    // let mut s: State<'_, '_, I, D> = State::new(stop_when_err, data, errors);
    // s.push(Box::new(root));
    // let mut finish = false;
    // let mut is_yield = false;
    // let mut yield_val: VecDeque<U> = VecDeque::new();
    // let i = DoLoopIter::new(move || -> Option<U> {
    //     loop {
    //         if !yield_val.is_empty() {
    //             let rv = yield_val.pop_front();
    //             if let Some(v) = rv {
    //                 return Some(v);
    //             }
    //         }
    //         if !is_yield {
    //             let r = yields(&mut s.data);
    //             if let Some(mut v) = r {
    //                 is_yield = true;
    //                 let rv = v.pop_front();
    //                 if !v.is_empty() {
    //                     yield_val = v;
    //                 }
    //                 if let Some(v) = rv {
    //                     return Some(v);
    //                 }
    //             }
    //         }
    //         is_yield = false;

    //         if !s.queue.is_empty() {
    //             let mut q = s.queue.pop().unwrap();
    //             q(&mut s);
    //             continue;
    //         }

    //         if finish {
    //             break;
    //         }

    //         let c = next(&mut s.data);
    //         if c.is_none() {
    //             finish = true;
    //             let r = call(&mut s, Default::default(), false);
    //             if r.is_none() {
    //                 break;
    //             }
    //             continue;
    //         }
    //         let c: RecunsResult<I> = c.unwrap();
    //         let c = match c {
    //             Ok(c) => c,
    //             Err(err) => {
    //                 s.errors.push(err);
    //                 return None;
    //             }
    //         };

    //         let r = call(&mut s, c, false);
    //         if r.is_none() {
    //             break;
    //         }
    //     }
    //     None
    // });
    // i
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

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        (self.f)()
    }
}
impl<F> Debug for DoLoopIter<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("DoLoopIter").field("f", &"...").finish()
    }
}

#[macro_export]
macro_rules! try_ret {
    { $e:expr } => {
        if let Some(v) = $e {
            return v;
        }
    };
}
