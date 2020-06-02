use crate::observer::{complete_proxy_impl, error_proxy_impl, next_proxy_impl};

use crate::prelude::*;
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Clone)]
pub struct SwitchOnNextOp<S, InnerItem> {
  pub(crate) source: S,
  pub(crate) p: PhantomData<InnerItem>,
}

impl<S, InnerItem> Observable for SwitchOnNextOp<S, InnerItem>
where
  S: Observable,
{
  type Item = InnerItem;
  type Err = ();
}

impl<'a, SourceObservable, InnerObservable, InnerItem> LocalObservable<'a>
  for SwitchOnNextOp<SourceObservable, InnerItem>
where
  SourceObservable: LocalObservable<'a, Item = InnerObservable> + 'a,
  InnerObservable: LocalObservable<'a, Item = InnerItem, Err = ()> + 'a,
{
  type Unsub = LocalSubscription;

  fn actual_subscribe<O: Observer<Self::Item, Self::Err> + 'a>(
    self,
    subscriber: Subscriber<O, LocalSubscription>,
  ) -> Self::Unsub {
    let mut subscription = subscriber.subscription;
    let mut inner_subscription = LocalSubscription::default();
    // We need to "hand out" ownership of the observer multiple times (whenever
    // the outer observable emits a new inner observable), so we need to
    // make it shared.
    let subscriber = Subscriber {
      observer: SwitchOnNextObserver {
        observer: Rc::new(RefCell::new(subscriber.observer)),
        subscription: subscription.clone(),
        inner_subscription: inner_subscription.clone(),
        one_is_complete: Rc::new(Cell::new(false)),
      },
      subscription: subscription.clone(),
    };
    subscription.add(self.source.actual_subscribe(subscriber));
    subscription
  }
}

#[derive(Clone)]
pub struct SwitchOnNextObserver<O, Unsub> {
  observer: Rc<RefCell<O>>,
  subscription: Unsub,
  inner_subscription: LocalSubscription,
  one_is_complete: Rc<Cell<bool>>,
}

#[derive(Clone)]
struct InnerObserver<O> {
  observer: Rc<RefCell<O>>,
  one_is_complete: Rc<Cell<bool>>,
}

impl<O, Item, Err> Observer<Item, Err> for InnerObserver<O>
where
  O: Observer<Item, Err>,
{
  #[inline]
  fn next(&mut self, value: Item) { self.observer.next(value); }

  error_proxy_impl!(Err, observer);

  #[inline]
  fn complete(&mut self) {
    if self.one_is_complete.replace(true) {
      self.observer.complete();
    }
  }
}

// TODO Is `Item` bound correct or too restrictive?
impl<'a, Item, Err, InnerItem, O, Unsub> Observer<Item, Err>
  for SwitchOnNextObserver<O, Unsub>
where
  O: Observer<InnerItem, ()> + 'a,
  Unsub: SubscriptionLike,
  Item: LocalObservable<'a, Item = InnerItem, Err = ()>,
{
  #[inline]
  fn next(&mut self, value: Item) {
    // Unsubscribe from previous inner observable (if any)
    self.inner_subscription.unsubscribe();
    // Create a new inner subscription
    let inner_subscription = LocalSubscription::default();
    self.inner_subscription = inner_subscription.clone();
    // Reset completion
    self.one_is_complete.set(false);
    // Subscribe
    value.actual_subscribe(Subscriber {
      observer: InnerObserver {
        observer: self.observer.clone(),
        one_is_complete: self.one_is_complete.clone(),
      },
      subscription: inner_subscription,
    });
  }

  #[inline]
  fn error(&mut self, err: Err) {
    // TODO Unsubscribe inner observable and emit error
    // self.observer.error(err);
  }

  #[inline]
  fn complete(&mut self) {
    if self.one_is_complete.replace(true) {
      self.observer.complete();
    }
  }
}

#[cfg(test)]
mod test {
  use crate::prelude::*;

  #[test]
  fn base_function() {
    let mut subject: LocalSubject<i32, ()> = LocalSubject::new();
    let ranges = subject.clone().map(|i| observable::from_iter(i..(i + 3)));
    ranges.switch_on_next().subscribe_complete(
      |i| {
        println!("{}", i);
      },
      || println!("complete"),
    );
    subject.next(0);
    subject.next(10);
    subject.next(100);
    subject.complete();
  }

  #[test]
  fn completion_details() {
    let mut subject: LocalSubject<_, ()> = LocalSubject::new();
    let mut subject_a: LocalSubject<_, ()> = LocalSubject::new();
    let mut subject_a_clone = subject_a.clone();
    let mut subject_b: LocalSubject<_, ()> = LocalSubject::new();
    let mut subject_b_clone = subject_b.clone();
    let mut subject_c: LocalSubject<_, ()> = LocalSubject::new();
    let mut subject_c_clone = subject_c.clone();
    let ranges = subject.clone().map(move |i| match i {
      "a" => subject_a_clone.clone(),
      "b" => subject_b_clone.clone(),
      _ => subject_c_clone.clone(),
    });
    ranges.switch_on_next().subscribe_complete(
      |i| {
        println!("{}", i);
      },
      || println!("complete"),
    );
    // a
    subject.next("a");
    subject_a.next("a1");
    subject_a.next("a2");
    // b
    subject.next("b");
    subject_a.next("a3");
    subject_a.complete();
    subject_b.next("b1");
    subject_b.next("b2");
    subject_b.complete();
    subject_c.next("c1");
    // c
    subject.next("c");
    subject_c.next("c2");
    subject.complete();
    subject_c.next("c3");
    subject_c.complete();
  }

  // #[test]
  // fn base_function() {
  //   let mut last_next_arg = None;
  //   let mut next_count = 0;
  //   let mut completed_count = 0;
  //   {
  //     let mut notifier = Subject::new();
  //     let mut source = Subject::new();
  //     source
  //       .clone()
  //       .take_until(notifier.clone())
  //       .subscribe_complete(
  //         |i| {
  //           last_next_arg = Some(i);
  //           next_count += 1;
  //         },
  //         || {
  //           completed_count += 1;
  //         },
  //       );
  //     source.next(5);
  //     notifier.next(());
  //     source.next(6);
  //     notifier.complete();
  //     source.complete();
  //   }
  //   assert_eq!(next_count, 1);
  //   assert_eq!(last_next_arg, Some(5));
  //   assert_eq!(completed_count, 1);
  // }
  //
  // #[test]
  // fn into_shared() {
  //   let last_next_arg = Arc::new(Mutex::new(None));
  //   let last_next_arg_mirror = last_next_arg.clone();
  //   let next_count = Arc::new(Mutex::new(0));
  //   let next_count_mirror = next_count.clone();
  //   let completed_count = Arc::new(Mutex::new(0));
  //   let completed_count_mirror = completed_count.clone();
  //   let mut notifier = Subject::new();
  //   let mut source = Subject::new();
  //   source
  //     .clone()
  //     .take_until(notifier.clone())
  //     .to_shared()
  //     .subscribe_complete(
  //       move |i| {
  //         *last_next_arg.lock().unwrap() = Some(i);
  //         *next_count.lock().unwrap() += 1;
  //       },
  //       move || {
  //         *completed_count.lock().unwrap() += 1;
  //       },
  //     );
  //   source.next(5);
  //   notifier.next(());
  //   source.next(6);
  //   assert_eq!(*next_count_mirror.lock().unwrap(), 1);
  //   assert_eq!(*last_next_arg_mirror.lock().unwrap(), Some(5));
  //   assert_eq!(*completed_count_mirror.lock().unwrap(), 1);
  // }
}
