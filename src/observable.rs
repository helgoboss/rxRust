mod trivial;
pub use trivial::*;

mod from;
pub use from::*;

pub(crate) mod from_future;
pub use from_future::{from_future, from_future_with_err};

pub(crate) mod interval;
pub use interval::{interval, interval_at};

pub(crate) mod connectable_observable;
pub use connectable_observable::{Connect, ConnectableObservable};

pub mod from_fn;
pub use from_fn::{create, ObservableFromFn};

mod observable_all;
pub use observable_all::*;
mod observable_err;
pub use observable_err::*;
mod observable_next;
pub use observable_next::*;
mod observable_comp;
pub use observable_comp::*;

use crate::prelude::*;
use std::sync::{Arc, Mutex};
use crate::ops::take_last::TakeLastOp;
use std::marker::PhantomData;

pub trait IntoShared {
  type Shared: Sync + Send + 'static;
  fn to_shared(self) -> Self::Shared;
}

pub trait Observable<O, U: SubscriptionLike> {
  /// A type implementing [`SubscriptionLike`]
  type Unsub: SubscriptionLike;
  fn actual_subscribe(self, subscriber: Subscriber<O, U>) -> Self::Unsub;

  /// Emits only the last `count` values emitted by the source Observable.
  ///
  /// `take_last` returns an Observable that emits only the last `count` values
  /// emitted by the source Observable. If the source emits fewer than `count`
  /// values then all of its values are emitted.
  /// It will not emit values until source Observable complete.
  ///
  /// # Example
  /// Take the last 5 seconds of an infinite 1-second interval Observable
  ///
  /// ```
  /// # use rxrust::{
  ///   ops::{TakeLast}, prelude::*,
  /// };
  ///
  /// observable::from_iter(0..10).take_last(5).subscribe(|v| println!("{}", v));
  ///

  /// // print logs:
  /// // 5
  /// // 6
  /// // 7
  /// // 8
  /// // 9
  /// ```
  ///
  fn take_last<Item>(self, count: usize) -> TakeLastOp<Self, Item>
    where
        Self: Sized,
  {
    TakeLastOp {
      source: self,
      count,
      _p: PhantomData,
    }
  }
}

impl<Item, Err> IntoShared for Box<dyn Observer<Item, Err> + Send + Sync>
where
  Item: 'static,
  Err: 'static,
{
  type Shared = Self;
  #[inline(always)]
  fn to_shared(self) -> Self::Shared { self }
}
impl<S> IntoShared for Arc<Mutex<S>>
where
  S: Send + Sync + 'static,
{
  type Shared = Self;
  #[inline(always)]
  fn to_shared(self) -> Self::Shared { self }
}
