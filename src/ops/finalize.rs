use crate::observer::{complete_proxy_impl, error_proxy_impl};
use crate::prelude::*;

#[derive(Clone)]
pub struct FinalizeOp<S, F> {
  pub(crate) source: S,
  pub(crate) func: F,
}

impl<S, F> Observable for FinalizeOp<S, F>
where
  S: Observable,
  F: Fn(),
{
  type Item = S::Item;
  type Err = S::Err;
}

impl<'a, S, F> LocalObservable<'a> for FinalizeOp<S, F>
where
  S: LocalObservable<'a>,
  F: Fn() + 'static,
{
  type Unsub = LocalSubscription;

  fn actual_subscribe<O: Observer<Self::Item, Self::Err> + 'a>(
    self,
    subscriber: Subscriber<O, LocalSubscription>,
  ) -> Self::Unsub {
    let mut subscription = LocalSubscription::default();
    subscription.add(self.source.actual_subscribe(subscriber));
    subscription.add(Finalizer {
      s: LocalSubscription::default(),
      f: self.func,
    });
    subscription
  }
}

struct Finalizer<F> {
  s: LocalSubscription,
  f: F,
}

impl<F> SubscriptionLike for Finalizer<F>
where
  F: Fn(),
{
  fn unsubscribe(&mut self) {
    self.s.unsubscribe();
    (self.f)()
  }

  fn is_closed(&self) -> bool { self.s.is_closed() }

  fn inner_addr(&self) -> *const () { self.s.inner_addr() }
}

#[cfg(test)]
mod test {
  use crate::prelude::*;

  #[test]
  fn basics() {
    let o = observable::of(1);
    o.finalize(|| println!("finalized"))
      .subscribe(|i| println!("{}", i));
  }
}
