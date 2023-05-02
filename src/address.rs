use std::{error::Error, fmt::Display, time::Duration};

use tokio::{
    sync::{
        mpsc::{self, WeakSender},
        oneshot,
    },
    task::JoinHandle,
    time::Instant,
};

use crate::{
    handler::{Envelope, MessageWithSender, UnpackableResult},
    Actor, Handler,
};

pub struct Address<A: Actor> {
    pub(crate) tx: mpsc::Sender<Box<dyn Envelope<A> + Send>>,
}

pub struct UnboundedAddress<A: Actor> {
    pub(crate) tx: mpsc::UnboundedSender<Box<dyn Envelope<A> + Send>>,
}

impl<A: Actor> Clone for Address<A> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<A: Actor> Clone for UnboundedAddress<A> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<A: Actor> Address<A> {
    // Sends a message to the actor
    pub async fn send<M: Message + 'static>(&self, message: M) -> Result<M::Result, ActorSendError>
    where
        A: Handler<M>,
    {
        let (tx, rx) = oneshot::channel();

        let packed = MessageWithSender { message, tx };

        self.tx
            .send(Box::new(packed))
            .await
            .map_err(|_| ActorSendError::FailedToDeliver)?;

        rx.await.map_err(|_| ActorSendError::FailedToGetResponse)
    }

    /// Sends a message and unpacks the result
    ///
    /// Simplifies operations with some common kinds of results. Unpacks:
    /// - `tokio::oneshot::Receiver<T>` into `T`
    /// - `Result<tokio::oneshot::Receiver<Result<T, E>, E>` into `Result<T, E>`
    ///
    /// It could be useful when you need to return `Receiver<T>` from handler
    /// immediately unblocking actors message loop to send `T` later.
    pub async fn send_unpack<M: Message + 'static, R>(
        &self,
        message: M,
    ) -> Result<R, ActorSendError>
    where
        A: Handler<M>,
        M::Result: UnpackableResult<UnpackedResult = R>,
    {
        match self.send(message).await {
            Ok(v) => v.unpack_result().await,
            Err(err) => Err(err),
        }
    }

    /// Returns `true` if the actor do not receive messages any more.
    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }

    /// Converts the `Address` to a [`WeakAddress`] that does not count
    /// towards RAII semantics, i.e. if all `Address` instances of the
    /// actor were dropped and only `WeakAddress` instances remain,
    /// the actor is stopped.
    pub fn downgrade(&self) -> WeakAddress<A> {
        WeakAddress {
            tx: self.tx.downgrade(),
        }
    }

    /// Returns `true` if actor is steel receiving messages.
    ///
    /// The opposite of `is_closed`
    pub fn is_connected(&self) -> bool {
        !self.tx.is_closed()
    }
}

pub struct WeakAddress<A: Actor> {
    tx: WeakSender<Box<dyn Envelope<A> + Send>>,
}

impl<A: Actor> WeakAddress<A> {
    pub fn upgrade(&self) -> Option<Address<A>> {
        self.tx.upgrade().map(|tx| Address { tx })
    }
}

impl<A: Actor> Clone for WeakAddress<A> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<A: Actor> UnboundedAddress<A> {
    pub async fn send<M: Message + 'static>(&self, message: M) -> Result<M::Result, ActorSendError>
    where
        A: Handler<M>,
    {
        let (tx, rx) = oneshot::channel();

        let packed = MessageWithSender { message, tx };

        self.tx
            .send(Box::new(packed))
            .map_err(|_| ActorSendError::FailedToDeliver)?;

        rx.await.map_err(|_| ActorSendError::FailedToGetResponse)
    }

    /// Sends a message and unpacks the result
    ///
    /// Simplifies operations with some common kinds of results
    pub async fn send_unpack<M: Message + 'static, R>(
        &self,
        message: M,
    ) -> Result<R, ActorSendError>
    where
        A: Handler<M>,
        M::Result: UnpackableResult<UnpackedResult = R>,
    {
        match self.send(message).await {
            Ok(v) => v.unpack_result().await,
            Err(err) => Err(err),
        }
    }

    pub fn notify<M: Envelope<A> + 'static + Send>(
        &self,
        message: M,
    ) -> Result<(), FailedToDeliver> {
        self.tx.send(Box::new(message)).map_err(|_| FailedToDeliver)
    }

    pub fn notify_later<M: Envelope<A> + 'static + Send>(&self, message: M, after: Duration) -> () {
        let address = self.clone();

        tokio::spawn(async move {
            tokio::time::sleep(after).await;

            let _ = address.notify(message);
        });
    }

    pub fn notify_at<M: Envelope<A> + 'static + Send>(&self, message: M, at: Instant) -> () {
        let address = self.clone();

        tokio::spawn(async move {
            tokio::time::sleep_until(at).await;

            let _ = address.notify(message);
        });
    }

    pub fn notify_interval<M: Message + 'static, F: (Fn() -> M) + Send + 'static>(
        &self,
        create_message: F,
        period: Duration,
    ) -> JoinHandle<()>
    where
        A: Handler<M>,
    {
        let address = self.clone();
        let mut interval = tokio::time::interval(period);

        tokio::spawn(async move {
            loop {
                interval.tick().await;

                let message = create_message();

                if let Err(_) = address.send(message).await {
                    break;
                }
            }
        })
    }

    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }
}

pub trait Message
where
    Self: Send,
{
    type Result: Send;
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum ActorSendError {
    FailedToDeliver,

    FailedToGetResponse,
}

impl Display for ActorSendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActorSendError::FailedToDeliver => write!(f, "Failed to deliver message to the actor"),
            ActorSendError::FailedToGetResponse => {
                write!(f, "Failed to get response from the actor")
            }
        }
    }
}

impl Error for ActorSendError {}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct FailedToDeliver;

impl Display for FailedToDeliver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to deliver notification")
    }
}

impl Error for FailedToDeliver {}
