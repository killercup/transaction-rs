//! # Zero-cost transactions in Rust
//! This crate abstracts over transactions like STM, SQL transactions and so on.
//! It is also composable via combinators and do DI of transactions.


use std::marker::PhantomData;

#[cfg(feature="mdo")]
pub mod mdo;

pub mod prelude {
    pub use super::{Transaction, result, ok, err, lazy, with_ctx};
}

/// An abstract transaction.
/// Transactions sharing the same `Ctx` can be composed with combinators.
pub trait Transaction<Ctx> {
    /// The return type of the transaction
    type Item;
    /// The error type of the transaction
    type Err;

    /// Run the transaction. This will called by transaction runner rather than user by hand.
    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err>;

    /// Box the transaction
    fn boxed<'a>(self) -> Box<Transaction<Ctx, Item = Self::Item, Err = Self::Err> + 'a>
        where Self: Sized + 'a
    {
        Box::new(self)
    }

    /// Take the previous result of computation and do another computation
    fn then<F, B, Tx2>(self, f: F) -> Then<Self, F, Tx2>
        where Tx2: Transaction<Ctx, Item = B, Err = Self::Err>,
              F: Fn(Result<Self::Item, Self::Err>) -> Tx2,
              Self: Sized
    {
        Then {
            tx: self,
            f: f,
            _phantom: PhantomData,
        }
    }

    /// transform the previous successful value
    fn map<F, B>(self, f: F) -> Map<Self, F>
        where F: Fn(Self::Item) -> B,
              Self: Sized
    {
        Map { tx: self, f: f }
    }



    /// Take the previous successful value of computation and do another computation
    fn and_then<F, B>(self, f: F) -> AndThen<Self, F, B>
        where B: Transaction<Ctx, Err = Self::Err>,
              F: Fn(Self::Item) -> B,
              Self: Sized
    {
        AndThen {
            tx: self,
            f: f,
            _phantom: PhantomData,
        }
    }

    /// transform the previous error value
    fn map_err<F, B>(self, f: F) -> MapErr<Self, F>
        where F: Fn(Self::Err) -> B,
              Self: Sized
    {
        MapErr { tx: self, f: f }
    }


    /// Take the previous error value of computation and do another computation.
    /// This may be used falling back
    fn or_else<F, B>(self, f: F) -> OrElse<Self, F, B>
        where B: Transaction<Ctx, Item = Self::Item>,
              F: Fn(Self::Err) -> B,
              Self: Sized
    {
        OrElse {
            tx: self,
            f: f,
            _phantom: PhantomData,
        }
    }

    /// Abort the transaction
    fn abort<T, F>(self, f: F) -> Abort<Self, T, F>
        where F: Fn(Self::Item) -> Self::Err,
              Self: Sized
    {
        Abort {
            tx: self,
            f: f,
            _phantom: PhantomData,
        }
    }

    /// Try to abort the transaction
    fn try_abort<F, B>(self, f: F) -> TryAbort<Self, F, B>
        where F: Fn(Self::Item) -> Result<B, Self::Err>,
              Self: Sized
    {
        TryAbort {
            tx: self,
            f: f,
            _phantom: PhantomData,
        }
    }

    /// Recover the transaction
    fn recover<T, F>(self, f: F) -> Recover<Self, T, F>
        where F: Fn(Self::Item) -> Self::Err,
              Self: Sized
    {
        Recover {
            tx: self,
            f: f,
            _phantom: PhantomData,
        }
    }

    /// Try to recover the transaction
    fn try_recover<F, B>(self, f: F) -> TryRecover<Self, F, B>
        where F: Fn(Self::Item) -> Result<B, Self::Err>,
              Self: Sized
    {
        TryRecover {
            tx: self,
            f: f,
            _phantom: PhantomData,
        }
    }

    /// join 2 indepndant transactions
    fn join<B>(self, b: B) -> Join<Self, B>
        where B: Transaction<Ctx, Err = Self::Err>,
              Self: Sized
    {
        Join { tx1: self, tx2: b }
    }

    /// join 3 indepndant transactions
    fn join3<B, C>(self, b: B, c: C) -> Join3<Self, B, C>
        where B: Transaction<Ctx, Err = Self::Err>,
              C: Transaction<Ctx, Err = Self::Err>,
              Self: Sized
    {
        Join3 {
            tx1: self,
            tx2: b,
            tx3: c,
        }
    }

    /// join 4 indepndant transactions
    fn join4<B, C, D>(self, b: B, c: C, d: D) -> Join4<Self, B, C, D>
        where B: Transaction<Ctx, Err = Self::Err>,
              C: Transaction<Ctx, Err = Self::Err>,
              D: Transaction<Ctx, Err = Self::Err>,
              Self: Sized
    {
        Join4 {
            tx1: self,
            tx2: b,
            tx3: c,
            tx4: d,
        }
    }
    // repeat
    // retry
}

/// Not used for now.
pub trait IntoTransaction {
    type Tx: Transaction<Self::Ctx, Item = Self::Item, Err = Self::Err>;
    type Ctx;
    type Err;
    type Item;

    fn into_transaction(self) -> Self::Tx;
}


/// Take a result and make a leaf transaction value.
pub fn result<T, E>(r: Result<T, E>) -> TxResult<T, E> {
    TxResult { r: r }
}

/// make a successful transaction value.
pub fn ok<T, E>(t: T) -> TxOk<T, E> {
    TxOk {
        ok: t,
        _phantom: PhantomData,
    }
}

/// make a error transaction value.
pub fn err<T, E>(e: E) -> TxErr<T, E> {
    TxErr {
        err: e,
        _phantom: PhantomData,
    }
}

/// lazy evaluated transaction value.
/// Note that inner function can be called many times.
pub fn lazy<F, T, E>(f: F) -> Lazy<F>
    where F: Fn() -> Result<T, E>
{
    Lazy { f: f }
}

/// Receive the context from the executing transaction and perform computation.
pub fn with_ctx<Ctx, F, T, E>(f: F) -> WithCtx<F>
    where F: Fn(&mut Ctx) -> Result<T, E>
{
    WithCtx { f: f }
}

/// The result of `then`
#[derive(Debug)]
pub struct Then<Tx1, F, Tx2> {
    tx: Tx1,
    f: F,
    _phantom: PhantomData<Tx2>,
}

/// The result of `map`
#[derive(Debug)]
pub struct Map<Tx, F> {
    tx: Tx,
    f: F,
}


/// The result of `and_then`
#[derive(Debug)]
pub struct AndThen<Tx1, F, Tx2> {
    tx: Tx1,
    f: F,
    _phantom: PhantomData<Tx2>,
}


/// The result of `map_err`
#[derive(Debug)]
pub struct MapErr<Tx, F> {
    tx: Tx,
    f: F,
}

/// The result of `or_else`
#[derive(Debug)]
pub struct OrElse<Tx1, F, Tx2> {
    tx: Tx1,
    f: F,
    _phantom: PhantomData<Tx2>,
}

/// The result of `abort`
#[derive(Debug)]
pub struct Abort<Tx, T, F> {
    tx: Tx,
    f: F,
    _phantom: PhantomData<T>,
}

/// The result of `try_abort`
#[derive(Debug)]
pub struct TryAbort<Tx, F, B> {
    tx: Tx,
    f: F,
    _phantom: PhantomData<B>,
}


/// The result of `recover`
#[derive(Debug)]
pub struct Recover<Tx, T, F> {
    tx: Tx,
    f: F,
    _phantom: PhantomData<T>,
}

/// The result of `try_recover`
#[derive(Debug)]
pub struct TryRecover<Tx, F, B> {
    tx: Tx,
    f: F,
    _phantom: PhantomData<B>,
}

/// The result of `join`
#[derive(Debug)]
pub struct Join<Tx1, Tx2> {
    tx1: Tx1,
    tx2: Tx2,
}

/// The result of `join3`
#[derive(Debug)]
pub struct Join3<Tx1, Tx2, Tx3> {
    tx1: Tx1,
    tx2: Tx2,
    tx3: Tx3,
}

/// The result of `join4`
#[derive(Debug)]
pub struct Join4<Tx1, Tx2, Tx3, Tx4> {
    tx1: Tx1,
    tx2: Tx2,
    tx3: Tx3,
    tx4: Tx4,
}


/// The result of `result`
#[derive(Debug)]
pub struct TxResult<T, E> {
    r: Result<T, E>,
}

/// The result of `ok`
#[derive(Debug)]
pub struct TxOk<T, E> {
    ok: T,
    _phantom: PhantomData<E>,
}

/// The result of `err`
#[derive(Debug)]
pub struct TxErr<T, E> {
    err: E,
    _phantom: PhantomData<T>,
}

/// The result of `lazy`
#[derive(Debug)]
pub struct Lazy<F> {
    f: F,
}

/// The result of `with_ctx`
#[derive(Debug)]
pub struct WithCtx<F> {
    f: F,
}

impl<Ctx, Tx, U, F> Transaction<Ctx> for Map<Tx, F>
    where Tx: Transaction<Ctx>,
          F: Fn(Tx::Item) -> U
{
    type Item = U;
    type Err = Tx::Err;

    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        let &Map { ref tx, ref f } = self;
        tx.run(ctx).map(f)
    }
}

impl<Ctx, Tx, Tx2, F> Transaction<Ctx> for Then<Tx, F, Tx2>
    where Tx2: Transaction<Ctx, Err = Tx::Err>,
          Tx: Transaction<Ctx>,
          F: Fn(Result<Tx::Item, Tx::Err>) -> Tx2
{
    type Item = Tx2::Item;
    type Err = Tx2::Err;

    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        let &Then { ref tx, ref f, .. } = self;
        f(tx.run(ctx)).run(ctx)
    }
}


impl<Ctx, Tx, Tx2, F> Transaction<Ctx> for AndThen<Tx, F, Tx2>
    where Tx2: Transaction<Ctx, Err = Tx::Err>,
          Tx: Transaction<Ctx>,
          F: Fn(Tx::Item) -> Tx2
{
    type Item = Tx2::Item;
    type Err = Tx2::Err;

    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        let &AndThen { ref tx, ref f, .. } = self;
        tx.run(ctx).and_then(|item| f(item).run(ctx))
    }
}


impl<Ctx, E, Tx, F> Transaction<Ctx> for MapErr<Tx, F>
    where Tx: Transaction<Ctx>,
          F: Fn(Tx::Err) -> E
{
    type Item = Tx::Item;
    type Err = E;

    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        let &MapErr { ref tx, ref f } = self;
        tx.run(ctx).map_err(f)
    }
}


impl<Ctx, Tx, Tx2, F> Transaction<Ctx> for OrElse<Tx, F, Tx2>
    where Tx2: Transaction<Ctx, Item = Tx::Item>,
          Tx: Transaction<Ctx>,
          F: Fn(Tx::Err) -> Tx2
{
    type Item = Tx2::Item;
    type Err = Tx2::Err;

    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        let &OrElse { ref tx, ref f, .. } = self;
        tx.run(ctx).or_else(|item| f(item).run(ctx))
    }
}


impl<Ctx, Tx, F, T> Transaction<Ctx> for Abort<Tx, T, F>
    where Tx: Transaction<Ctx>,
          F: Fn(Tx::Item) -> Tx::Err
{
    type Item = T;
    type Err = Tx::Err;

    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        let &Abort { ref tx, ref f, .. } = self;
        match tx.run(ctx) {
            Ok(r) => Err(f(r)),
            Err(e) => Err(e),
        }
    }
}

impl<Ctx, Tx, F, B> Transaction<Ctx> for TryAbort<Tx, F, B>
    where Tx: Transaction<Ctx>,
          F: Fn(Tx::Item) -> Result<B, Tx::Err>
{
    type Item = B;
    type Err = Tx::Err;

    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        let &TryAbort { ref tx, ref f, .. } = self;
        match tx.run(ctx) {
            Ok(r) => f(r),
            Err(e) => Err(e),
        }
    }
}



impl<Ctx, Tx, F, T> Transaction<Ctx> for Recover<Tx, T, F>
    where Tx: Transaction<Ctx>,
          F: Fn(Tx::Err) -> Tx::Item
{
    type Item = Tx::Item;
    type Err = Tx::Err;

    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        let &Recover { ref tx, ref f, .. } = self;
        match tx.run(ctx) {
            r @ Ok(_) => r,
            Err(e) => Ok(f(e)),
        }
    }
}

impl<Ctx, Tx, F, B> Transaction<Ctx> for TryRecover<Tx, F, B>
    where Tx: Transaction<Ctx>,
          F: Fn(Tx::Err) -> Result<Tx::Item, B>
{
    type Item = Tx::Item;
    type Err = B;

    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        let &TryRecover { ref tx, ref f, .. } = self;
        match tx.run(ctx) {
            Ok(r) => Ok(r),
            Err(e) => f(e),
        }
    }
}

impl<Ctx, Tx1, Tx2> Transaction<Ctx> for Join<Tx1, Tx2>
    where Tx1: Transaction<Ctx>,
          Tx2: Transaction<Ctx, Err = Tx1::Err>
{
    type Item = (Tx1::Item, Tx2::Item);
    type Err = Tx1::Err;

    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        let &Join { ref tx1, ref tx2 } = self;
        match (tx1.run(ctx), tx2.run(ctx)) {
            (Ok(r1), Ok(r2)) => Ok((r1, r2)),
            (Err(e), _) | (_, Err(e)) => Err(e),
        }
    }
}

impl<Ctx, Tx1, Tx2, Tx3> Transaction<Ctx> for Join3<Tx1, Tx2, Tx3>
    where Tx1: Transaction<Ctx>,
          Tx2: Transaction<Ctx, Err = Tx1::Err>,
          Tx3: Transaction<Ctx, Err = Tx1::Err>
{
    type Item = (Tx1::Item, Tx2::Item, Tx3::Item);
    type Err = Tx1::Err;

    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        let &Join3 {
                 ref tx1,
                 ref tx2,
                 ref tx3,
             } = self;
        match (tx1.run(ctx), tx2.run(ctx), tx3.run(ctx)) {
            (Ok(r1), Ok(r2), Ok(r3)) => Ok((r1, r2, r3)),
            (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => Err(e),
        }
    }
}

impl<Ctx, Tx1, Tx2, Tx3, Tx4> Transaction<Ctx> for Join4<Tx1, Tx2, Tx3, Tx4>
    where Tx1: Transaction<Ctx>,
          Tx2: Transaction<Ctx, Err = Tx1::Err>,
          Tx3: Transaction<Ctx, Err = Tx1::Err>,
          Tx4: Transaction<Ctx, Err = Tx1::Err>
{
    type Item = (Tx1::Item, Tx2::Item, Tx3::Item, Tx4::Item);
    type Err = Tx1::Err;

    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        let &Join4 {
                 ref tx1,
                 ref tx2,
                 ref tx3,
                 ref tx4,
             } = self;
        match (tx1.run(ctx), tx2.run(ctx), tx3.run(ctx), tx4.run(ctx)) {
            (Ok(r1), Ok(r2), Ok(r3), Ok(r4)) => Ok((r1, r2, r3, r4)),
            (Err(e), _, _, _) |
            (_, Err(e), _, _) |
            (_, _, Err(e), _) |
            (_, _, _, Err(e)) => Err(e),
        }
    }
}


impl<Ctx, T, E> Transaction<Ctx> for TxResult<T, E>
    where T: Clone,
          E: Clone
{
    type Item = T;
    type Err = E;
    fn run(&self, _ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        self.r.clone()
    }
}

impl<Ctx, T, E> Transaction<Ctx> for TxOk<T, E>
    where T: Clone
{
    type Item = T;
    type Err = E;
    fn run(&self, _ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        Ok(self.ok.clone())
    }
}

impl<Ctx, T, E> Transaction<Ctx> for TxErr<T, E>
    where E: Clone
{
    type Item = T;
    type Err = E;
    fn run(&self, _ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        Err(self.err.clone())
    }
}


impl<Ctx, T, E, F> Transaction<Ctx> for Lazy<F>
    where F: Fn() -> Result<T, E>
{
    type Item = T;
    type Err = E;
    fn run(&self, _ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        (self.f)()
    }
}

impl<Ctx, T, E, F> Transaction<Ctx> for WithCtx<F>
    where F: Fn(&mut Ctx) -> Result<T, E>
{
    type Item = T;
    type Err = E;
    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        (self.f)(ctx)
    }
}

impl<Ctx, T, E> Transaction<Ctx> for Fn(&mut Ctx) -> Result<T, E> {
    type Item = T;
    type Err = E;
    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        self(ctx)
    }
}


impl<T, Ctx> Transaction<Ctx> for Box<T>
    where T: ?Sized + Transaction<Ctx>
{
    type Item = T::Item;
    type Err = T::Err;
    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        (**self).run(ctx)
    }
}

impl<'a, T, Ctx> Transaction<Ctx> for &'a T
    where T: ?Sized + Transaction<Ctx>
{
    type Item = T::Item;
    type Err = T::Err;
    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        (**self).run(ctx)
    }
}
