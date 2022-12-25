use async_trait::async_trait;
use tokio::sync::oneshot;

use crate::{Actor, ActorSendError, Context, Message};

#[async_trait]
pub trait Handler<M: Message>
where
    Self: Actor,
{
    async fn handle(&mut self, message: M, context: &mut Context<Self>) -> M::Result;
}

pub struct MessageWithSender<M: Message> {
    pub message: M,
    pub tx: oneshot::Sender<M::Result>,
}

#[async_trait]
pub trait Envelope<A: Actor> {
    async fn handle(self: Box<Self>, actor: &mut A, context: &mut Context<A>);
}

#[async_trait]
impl<M: Message, A: Actor> Envelope<A> for MessageWithSender<M>
where
    A: Handler<M>,
{
    async fn handle(mut self: Box<Self>, actor: &mut A, context: &mut Context<A>) {
        tokio::select! {
          // Drop computation if receiver is no longer interested in it
          _ = self.tx.closed() => {}
          result = actor.handle(self.message, context) => {
            // It is OK if receiver is not interested in the response any more
            // and eventualy it was not captured earlier
            let _ = self.tx.send(result);
          }
        }
    }
}

#[async_trait]
impl<M: Message, A: Actor> Envelope<A> for M
where
    A: Handler<M>,
{
    async fn handle(mut self: Box<Self>, actor: &mut A, context: &mut Context<A>) {
        // It is just a notification, we are not interested in handling the result
        let _ = actor.handle(*self, context).await;
    }
}

#[async_trait]
pub trait UnpackableResult {
    type UnpackedResult;

    async fn unpack_result(self) -> Result<Self::UnpackedResult, ActorSendError>;
}

#[async_trait]
impl<T: Send> UnpackableResult for oneshot::Receiver<T> {
    type UnpackedResult = T;

    async fn unpack_result(self) -> Result<Self::UnpackedResult, ActorSendError> {
        self.await.map_err(|_| ActorSendError::FailedToGetResponse)
    }
}

#[async_trait]
impl<T: Send, E: Send> UnpackableResult for Result<oneshot::Receiver<Result<T, E>>, E> {
    type UnpackedResult = Result<T, E>;

    async fn unpack_result(self) -> Result<Self::UnpackedResult, ActorSendError> {
        match self {
            Ok(v) => v.await.map_err(|_| ActorSendError::FailedToGetResponse),
            Err(err) => Ok(Err(err)),
        }
    }
}
