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
  SourceObservable: LocalObservable<'a, Item = InnerObservable, Err = ()> + 'a,
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
pub struct SwitchOnNextObserver<O> {
  observer: Rc<RefCell<O>>,
  subscription: LocalSubscription,
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
impl<'a, Item, InnerItem, O> Observer<Item, ()> for SwitchOnNextObserver<O>
where
  O: Observer<InnerItem, ()> + 'a,
  Item: LocalObservable<'a, Item = InnerItem, Err = ()>,
{
  #[inline]
  fn next(&mut self, value: Item) {
    // Unsubscribe from previous inner observable (if any)
    self.inner_subscription.unsubscribe();
    // Create a new inner subscription
    let inner_subscription = LocalSubscription::default();
    self.inner_subscription = inner_subscription.clone();
    // Make the inner subscription end when the outer one ends
    self.subscription.add(inner_subscription.clone());
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
  fn error(&mut self, err: ()) {
    self.inner_subscription.unsubscribe();
    self.observer.error(err);
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
  use std::cell::RefCell;
  use std::rc::Rc;

  #[derive(Eq, PartialEq, Debug)]
  enum Event<Item, Err> {
    Next(Item),
    Error(Err),
    Complete,
  }

  #[derive(Default)]
  struct EventBuffer<Item, Err> {
    buffer: RefCell<Vec<Event<Item, Err>>>,
  }

  impl<Item, Err> EventBuffer<Item, Err> {
    fn next(&self, item: Item) {
      self.buffer.borrow_mut().push(Event::Next(item));
    }

    fn error(&self, err: Err) {
      self.buffer.borrow_mut().push(Event::Error(err));
    }

    fn complete(&self) { self.buffer.borrow_mut().push(Event::Complete); }

    /// Empties buffer and returns current content.
    fn pop(&self) -> Vec<Event<Item, Err>> {
      self.buffer.replace(Default::default())
    }
  }

  #[test]
  fn base_function() {
    // Given
    let buffer: Rc<EventBuffer<i32, ()>> = Default::default();
    let mut s = LocalSubject::new();
    let ranges = s.clone().map(|i| observable::from_iter(i..(i + 3)));
    // When
    let bc1 = buffer.clone();
    let bc2 = buffer.clone();
    ranges
      .switch_on_next()
      .subscribe_complete(move |i| bc1.next(i), move || bc2.complete());
    // Then
    use Event::*;
    s.next(0);
    assert_eq!(buffer.pop().as_slice(), &[Next(0), Next(1), Next(2)]);
    s.next(10);
    assert_eq!(buffer.pop().as_slice(), &[Next(10), Next(11), Next(12)]);
    s.next(100);
    assert_eq!(buffer.pop().as_slice(), &[Next(100), Next(101), Next(102)]);
    s.complete();
    assert_eq!(buffer.pop().as_slice(), &[Complete]);
  }

  #[test]
  fn completion_details() {
    // Given
    let buffer: Rc<EventBuffer<i32, ()>> = Default::default();
    let mut s = LocalSubject::new();
    let ranges = s.clone().map(|i| observable::from_iter(i..(i + 3)));
    // When
    let bc1 = buffer.clone();
    let bc2 = buffer.clone();
    ranges
      .switch_on_next()
      .subscribe_complete(move |i| bc1.next(i), move || bc2.complete());
    // Then
    use Event::*;
    s.next(0);
    assert_eq!(buffer.pop().as_slice(), &[Next(0), Next(1), Next(2)]);
    s.next(10);
    assert_eq!(buffer.pop().as_slice(), &[Next(10), Next(11), Next(12)]);
    s.next(100);
    assert_eq!(buffer.pop().as_slice(), &[Next(100), Next(101), Next(102)]);
    s.complete();
    assert_eq!(buffer.pop().as_slice(), &[Complete]);
  }

  #[test]
  fn completion_details_old() {
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

  #[test]
  fn unsubscribe_details() {
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
    let mut subscription = ranges.switch_on_next().subscribe_complete(
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
    subscription.unsubscribe();
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
}
