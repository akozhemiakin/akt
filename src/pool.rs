use std::{ops::Deref, thread::available_parallelism};

use tokio::sync::mpsc;

use crate::{Actor, ActorSpawner, Address};

pub struct Pool<A: Actor> {
    actors_tx: mpsc::UnboundedSender<Address<A>>,
    actors_rx: mpsc::UnboundedReceiver<Address<A>>,
}

pub struct PoolAddress<A: Actor> {
    inner: Address<A>,
    actors_tx: mpsc::UnboundedSender<Address<A>>,
}

impl<A: Actor> Drop for PoolAddress<A> {
    fn drop(&mut self) {
        let _ = self.actors_tx.send(self.inner.clone());
    }
}

impl<A: Actor> Deref for PoolAddress<A> {
    type Target = Address<A>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<A: Actor> Pool<A> {
    pub fn with_spawner(spawner: ActorSpawner<A>, instances: usize) -> Pool<A> {
        let (tx, rx) = mpsc::unbounded_channel();

        for _ in 0..instances.into() {
            let _ = tx.send(spawner.spawn_run());
        }

        Pool {
            actors_tx: tx,
            actors_rx: rx,
        }
    }

    pub fn with_spawner_default(spawner: ActorSpawner<A>) -> Pool<A> {
        Self::with_spawner(spawner, available_parallelism().map_or(4, |v| v.into()))
    }

    pub async fn take(&mut self) -> PoolAddress<A> {
        // It seems safe to unwrap here because tx may be dropped only after
        // Pool itself is dropped
        PoolAddress {
            inner: self.actors_rx.recv().await.unwrap(),
            actors_tx: self.actors_tx.clone(),
        }
    }
}
