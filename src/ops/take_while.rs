use crate::observer::{complete_proxy_impl, error_proxy_impl};
use crate::prelude::*;

#[derive(Clone)]
pub struct TakeWhileOp<S, F> {
  pub(crate) source: S,
  pub(crate) callback: F,
}

#[doc(hidden)]
macro observable_impl(
  $subscription:ty, $source:ident, $($marker:ident +)* $lf: lifetime)
{
  type Unsub = $source::Unsub;
  fn actual_subscribe<O: Observer<Self::Item, Self::Err> + $($marker +)* $lf>(
    self,
    subscriber: Subscriber<O, $subscription>,
  ) -> Self::Unsub {
    let subscriber = Subscriber {
      observer: TakeWhileObserver {
        observer: subscriber.observer,
        subscription: subscriber.subscription.clone(),
        callback: self.callback,
      },
      subscription: subscriber.subscription,
    };
    self.source.actual_subscribe(subscriber)
  }
}

impl<S, F> Observable for TakeWhileOp<S, F>
where
  S: Observable,
  F: FnMut(&S::Item) -> bool,
{
  type Item = S::Item;
  type Err = S::Err;
}

impl<'a, S, F> LocalObservable<'a> for TakeWhileOp<S, F>
where
  S: LocalObservable<'a>,
  F: FnMut(&S::Item) -> bool + 'a,
{
  observable_impl!(LocalSubscription, S, 'a);
}

impl<S, F> SharedObservable for TakeWhileOp<S, F>
where
  S: SharedObservable,
  F: FnMut(&S::Item) -> bool + Send + Sync + 'static,
{
  observable_impl!(SharedSubscription, S, Send + Sync + 'static);
}

pub struct TakeWhileObserver<O, S, F> {
  observer: O,
  subscription: S,
  callback: F,
}

impl<O, U, Item, Err, F> Observer<Item, Err> for TakeWhileObserver<O, U, F>
where
  O: Observer<Item, Err>,
  U: SubscriptionLike,
  F: FnMut(&Item) -> bool,
{
  fn next(&mut self, value: Item) {
    if (self.callback)(&value) {
      self.observer.next(value);
    } else {
      self.observer.complete();
      self.subscription.unsubscribe();
    }
  }
  error_proxy_impl!(Err, observer);
  complete_proxy_impl!(observer);
}

#[cfg(test)]
mod test {
  use crate::prelude::*;

  #[test]
  fn base_function() {
    let mut completed = false;
    let mut next_count = 0;

    observable::from_iter(0..100)
      .take_while(|v| v < &5)
      .subscribe_complete(|_| next_count += 1, || completed = true);

    assert_eq!(next_count, 5);
    assert_eq!(completed, true);
  }

  #[test]
  fn take_while_support_fork() {
    let mut nc1 = 0;
    let mut nc2 = 0;
    {
      let take_while5 = observable::from_iter(0..100).take_while(|v| v < &5);
      let f1 = take_while5.clone();
      let f2 = take_while5;

      f1.take_while(|v| v < &5).subscribe(|_| nc1 += 1);
      f2.take_while(|v| v < &5).subscribe(|_| nc2 += 1);
    }
    assert_eq!(nc1, 5);
    assert_eq!(nc2, 5);
  }

  #[test]
  fn into_shared() {
    observable::from_iter(0..100)
      .take_while(|v| v < &5)
      .take_while(|v| v < &5)
      .to_shared()
      .subscribe(|_| {});
  }
}
