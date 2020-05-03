pub fn do_loop(&mut self) -> Result<(), Vec<Arc<dyn Error>>> {
    let this = unsafe { &mut *(self as *mut _) };
    loop {
        if let Some(ref cancel) = self.cancel {
            if cancel() {
                return Ok(());
            }
        }
        if let Some(ref mut v) = self.on_loop {
            v(this);
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
        let c: Result<I, Arc<dyn Error>> = c.unwrap();
        let c = match c {
            Ok(c) => c,
            Err(err) => {
                self.errors.push(err);
                return Err(self.errors.clone());
            }
        };
        let r = self.call(c);
        if r.is_none() {
            break;
        }
    }
    if !self.errors.is_empty() {
        return Err(self.errors.clone());
    }
    Ok(())
}
pub fn iter_loop<F, U>(&mut self, f: F) -> StateIter<'_, 'a, F, I, D>
where
    F: FnMut(&mut Self) -> Option<U>,
{
    StateIter::new(f, self)
}

fn call(&mut self, input: I) -> Option<()> {
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
