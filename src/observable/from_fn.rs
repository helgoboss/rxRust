use crate::prelude::*;
use ops::SharedOp;
use std::marker::PhantomData;

/// A representation of any set of values over any amount of time. This is the
/// most basic building block rxrust
///
pub struct ObservableFromFn<F, Item, Err>(F, PhantomData<(Item, Err)>);

macro impl_observable($t: ident) {
  unsafe impl<F, Item, Err> Send for $t<F, Item, Err> {}
  unsafe impl<F, Item, Err> Sync for $t<F, Item, Err> {}
  impl<F, Item, Err> Clone for $t<F, Item, Err>
  where
    F: Clone,
  {
    fn clone(&self) -> Self { $t(self.0.clone(), PhantomData) }
  }
}

impl_observable!(ObservableFromFn);

/// param `subscribe`: the function that is called when the Observable is
/// initially subscribed to. This function is given a Subscriber, to which
/// new values can be `next`ed, or an `error` method can be called to raise
/// an error, or `complete` can be called to notify of a successful
/// completion.
pub fn create<F, Item, Err, O, U>(
  subscribe: F,
) -> ObservableFromFn<F, Item, Err>
where
  F: FnOnce(Subscriber<O, U>),
  O: Observer<Item, Err>,
{
  ObservableFromFn(subscribe, PhantomData)
}

impl<F, Item, Err> Fork for ObservableFromFn<F, Item, Err>
where
  F: Clone,
{
  type Output = ForkObservable<F, Item, Err>;
  #[inline]
  fn fork(&self) -> Self::Output { ForkObservable(self.0.clone(), PhantomData) }
}

impl<F, O, U, Item, Err> Observable<O, U> for ObservableFromFn<F, Item, Err>
where
  F: FnOnce(Subscriber<O, U>),
  U: SubscriptionLike + Clone,
{
  type Unsub = U;
  fn actual_subscribe(self, subscriber: Subscriber<O, U>) -> Self::Unsub {
    let subscription = subscriber.subscription.clone();
    (self.0)(subscriber);
    subscription
  }
}

impl<F, Item, Err> IntoShared for ObservableFromFn<F, Item, Err>
where
  F: Send + Sync + 'static,
  Item: 'static,
  Err: 'static,
{
  type Shared = SharedOp<Self>;
  #[inline]
  fn to_shared(self) -> Self::Shared { SharedOp(self) }
}

pub struct ForkObservable<F, Item, Err>(F, PhantomData<(Item, Err)>);
impl_observable!(ForkObservable);

impl<F, Item, Err> IntoShared for ForkObservable<F, Item, Err>
where
  F: Send + Sync + 'static,
  Item: 'static,
  Err: 'static,
{
  type Shared = SharedForkObservable<F, Item, Err>;
  fn to_shared(self) -> Self::Shared {
    SharedForkObservable(self.0, PhantomData)
  }
}

impl<F, Item, Err> Fork for ForkObservable<F, Item, Err>
where
  F: Clone,
{
  type Output = Self;
  #[inline(always)]
  fn fork(&self) -> Self::Output { self.clone() }
}

impl<'a, F, Item, Err, O, U> Observable<O, U> for ForkObservable<F, Item, Err>
where
  O: Observer<Item, Err> + 'a,
  F: FnOnce(Subscriber<Box<dyn Observer<Item, Err> + 'a>, U>),
  U: SubscriptionLike + Clone + 'static,
{
  type Unsub = U;
  fn actual_subscribe(self, subscriber: Subscriber<O, U>) -> Self::Unsub {
    let observer: Box<dyn Observer<Item, Err> + 'a> =
      Box::new(subscriber.observer);

    let subscription = subscriber.subscription;
    let unsub = subscription.clone();
    (self.0)(Subscriber {
      observer,
      subscription,
    });
    unsub
  }
}

pub struct SharedForkObservable<F, Item, Err>(F, PhantomData<(Item, Err)>);
impl_observable!(SharedForkObservable);

impl<F, Item, Err> IntoShared for SharedForkObservable<F, Item, Err>
where
  Self: Send + Sync + 'static,
{
  type Shared = Self;
  #[inline(always)]
  fn to_shared(self) -> Self::Shared { self }
}

impl<F, Item, Err> Fork for SharedForkObservable<F, Item, Err>
where
  F: Clone,
{
  type Output = Self;
  #[inline(always)]
  fn fork(&self) -> Self::Output { self.clone() }
}

impl<F, Item, Err, O, U> Observable<O, U> for SharedForkObservable<F, Item, Err>
where
  O: IntoShared + Observer<Item, Err>,
  U: IntoShared + SubscriptionLike,
  O::Shared: Observer<Item, Err> + Send + Sync + 'static,
  U::Shared: SubscriptionLike + Clone,
  F: FnOnce(Subscriber<Box<dyn Observer<Item, Err> + Send + Sync>, U::Shared>),
{
  type Unsub = U::Shared;
  fn actual_subscribe(self, subscriber: Subscriber<O, U>) -> Self::Unsub {
    let subscription = subscriber.subscription.to_shared();
    let observer: Box<dyn Observer<Item, Err> + Send + Sync> =
      Box::new(subscriber.observer.to_shared());
    (self.0)(Subscriber {
      observer,
      subscription: subscription.clone(),
    });
    subscription
  }
}

#[cfg(test)]
mod test {
  use crate::ops::Fork;
  use crate::prelude::*;
  use std::sync::{Arc, Mutex};

  fn create_my_observable<O, U>() -> impl Observable<O, U>
    where
        O: Observer<i32, ()>,
        U: SubscriptionLike + Clone + 'static
  {
    observable::create(|mut subscriber| {
      subscriber.next(1);
      subscriber.next(2);
      subscriber.complete();
    })
  }

  #[test]
  fn sandbox() {
    create_my_observable().subscribe(|i| println!("{}", i));
  }

  #[test]
  fn proxy_call() {
    let next = Arc::new(Mutex::new(0));
    let err = Arc::new(Mutex::new(0));
    let complete = Arc::new(Mutex::new(0));
    let c_next = next.clone();
    let c_err = err.clone();
    let c_complete = complete.clone();

    observable::create(|mut subscriber| {
      subscriber.next(&1);
      subscriber.next(&2);
      subscriber.next(&3);
      subscriber.complete();
      subscriber.next(&3);
      subscriber.error(&"never dispatch error");
    })
    .to_shared()
    .subscribe_all(
      move |_| *next.lock().unwrap() += 1,
      move |_: &&str| *err.lock().unwrap() += 1,
      move || *complete.lock().unwrap() += 1,
    );

    assert_eq!(*c_next.lock().unwrap(), 3);
    assert_eq!(*c_complete.lock().unwrap(), 1);
    assert_eq!(*c_err.lock().unwrap(), 0);
  }
  #[test]
  fn support_fork() {
    let o = observable::create(|mut subscriber| {
      subscriber.next(&1);
      subscriber.next(&2);
      subscriber.next(&3);
      subscriber.next(&4);
    });
    let sum1 = Arc::new(Mutex::new(0));
    let sum2 = Arc::new(Mutex::new(0));
    let c_sum1 = sum1.clone();
    let c_sum2 = sum2.clone();
    o.fork().subscribe(move |v| *sum1.lock().unwrap() += v);
    o.fork().subscribe(move |v| *sum2.lock().unwrap() += v);

    assert_eq!(*c_sum1.lock().unwrap(), 10);
    assert_eq!(*c_sum2.lock().unwrap(), 10);
  }

  #[test]
  fn fork_and_share() {
    let observable = observable::empty();
    // shared after fork
    observable.fork().to_shared().subscribe(|_: &()| {});
    observable.fork().to_shared().subscribe(|_| {});

    // shared before fork
    let observable = observable::empty().to_shared();
    observable.fork().subscribe(|_: &()| {});
    observable.fork().subscribe(|_| {});
  }
}
