use crate::observer::{
    observer_next_proxy_impl, observer_complete_proxy_impl, observer_error_proxy_impl,
};
use crate::ops::SharedOp;
use crate::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;

pub trait TakeUntil {
    fn take_until<T>(self, trigger: T) -> TakeUntilOp<Self, T>
        where
            Self: Sized,
    {
        TakeUntilOp {
            source: self,
            trigger,
        }
    }
}

impl<O> TakeUntil for O {}

pub struct TakeUntilOp<S, T> {
    source: S,
    trigger: T,
}

//impl<S, T> IntoShared for TakeUntilOp<S, T>
//where
//  S: IntoShared,
//{
//  type Shared = SharedOp<TakeUntilOp<S::Shared>>;
//  fn to_shared(self) -> Self::Shared {
//    SharedOp(TakeUntilOp {
//      source: self.source.to_shared(),
//      count: self.count,
//    })
//  }
//}

impl<O, U, S, TS> Observable<O, U> for TakeUntilOp<S, TS>
    where
        S: Observable<TakeUntilObserver<O>, U>,
        U: SubscriptionLike + Clone + 'static,
        TS: Observable<TakeUntilTriggerObserver<O, U>, U>,

{
    type Unsub = S::Unsub;
    fn actual_subscribe(self, subscriber: Subscriber<O, U>) -> Self::Unsub {
        // We need to keep a reference to the observer from two places
        // TODO Would it be better to make one of them a weak pointer?
        let shared_observer = Rc::new(RefCell::new(subscriber.observer));
        let main_subscriber = Subscriber {
            observer: TakeUntilObserver {
                observer: shared_observer.clone(),
            },
            subscription: subscriber.subscription.clone(),
        };
        let trigger_subscriber = Subscriber {
            observer: TakeUntilTriggerObserver {
                subscription: subscriber.subscription.clone(),
                observer: shared_observer
            },
            subscription: subscriber.subscription,
        };
        self.trigger.actual_subscribe(trigger_subscriber);
        self.source.actual_subscribe(main_subscriber)
    }
}

pub struct TakeUntilObserver<O> {
    observer: Rc<RefCell<O>>,
}

pub struct TakeUntilTriggerObserver<O, S> {
    // Needs access to observer in order to call `complete` on it as soon as trigger fired
    observer: Rc<RefCell<O>>,
    // Needs to cancel subscription as soon as trigger fired
    subscription: S
}

impl<O, U, Item> ObserverNext<Item> for TakeUntilTriggerObserver<O, U>
    where
        O: ObserverComplete,
        U: SubscriptionLike,
{
    fn next(&mut self, value: Item) {
        self.observer.complete();
        self.subscription.unsubscribe();
    }
}

impl<O, U, Err> ObserverError<Err> for TakeUntilTriggerObserver<O, U>
    where
        O: ObserverError<Err>,
        U: SubscriptionLike,
{
    fn error(&mut self, err: Err) {
        self.observer.error(err);
        self.subscription.unsubscribe();
    }
}

impl<O, U> ObserverComplete for TakeUntilTriggerObserver<O, U>
{
    fn complete(&mut self) {
        // Do nothing
    }
}

//impl<S, ST> IntoShared for TakeUntilObserver<S, ST>
//where
//  S: IntoShared,
//  ST: IntoShared,
//{
//  type Shared = TakeUntilObserver<S::Shared, ST::Shared>;
//  fn to_shared(self) -> Self::Shared {
//    TakeUntilObserver {
//      observer: self.observer.to_shared(),
//      subscription: self.subscription.to_shared(),
//    }
//  }
//}

observer_next_proxy_impl!(TakeUntilObserver<O>, O, observer, <O>);
observer_error_proxy_impl!(TakeUntilObserver<O>, O, observer, <O>);
observer_complete_proxy_impl!(TakeUntilObserver<O>, O,  observer, <O>);

//impl<S> Fork for TakeUntilOp<S>
//where
//  S: Fork,
//{
//  type Output = TakeUntilOp<S::Output>;
//  fn fork(&self) -> Self::Output {
//    TakeUntilOp {
//      source: self.source.fork(),
//      count: self.count,
//    }
//  }
//}

#[cfg(test)]
mod test {
    use super::TakeUntil;
    use crate::prelude::*;

    #[test]
    fn base_function() {
        let mut trigger = Subject::local();
        let mut source = Subject::local();
        source.fork()
            .take_until(trigger.fork())
            .subscribe_all(
                |i| println!("Got {:?}", i),
                |_: ()| println!("Error"),
                || println!("Complete")
            );
        source.next(5.0);
        trigger.next(true);
        source.next(6.0);
    }

//  #[test]
//  fn take_until_support_fork() {
//    let mut nc1 = 0;
//    let mut nc2 = 0;
//    {
//      let take_until5 = observable::from_iter(0..100).take_until(5);
//      let f1 = take_until5.fork();
//      let f2 = take_until5.fork();
//
//      f1.take_until(5).fork().subscribe(|_| nc1 += 1);
//      f2.take_until(5).fork().subscribe(|_| nc2 += 1);
//    }
//    assert_eq!(nc1, 5);
//    assert_eq!(nc2, 5);
//  }
//
//  #[test]
//  fn into_shared() {
//    observable::from_iter(0..100)
//      .take_until(5)
//      .take_until(5)
//      .to_shared()
//      .subscribe(|_| {});
//  }
}
