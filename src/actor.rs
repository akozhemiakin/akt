use async_trait::async_trait;

use tokio::{
    select,
    sync::mpsc::{self},
};

use crate::{
    address::{Address, UnboundedAddress},
    handler::Envelope,
    ActorState, Context,
};

/// Core trait that should be implemented for each Actor.
#[async_trait]
pub trait Actor: Send + Sized + 'static {
    /// Runs actor consuming it and returning its address
    fn run(self) -> Address<Self> {
        // Public mailbox is bounded
        let (addr_tx, mut addr_rx) = mpsc::channel::<Box<dyn Envelope<Self> + Send>>(16);

        // Private mailbox is unbounded
        let (private_addr_tx, mut private_addr_rx) =
            mpsc::unbounded_channel::<Box<dyn Envelope<Self> + Send>>();

        // Public address
        // Intended to be used by anyone
        let address = Address { tx: addr_tx };

        // Private address
        // Intended to be used by actor that owned it and actors spawned and controlled by it
        let private_address = UnboundedAddress {
            tx: private_addr_tx,
        };

        let weak_address = address.downgrade();

        tokio::spawn(async move {
            let mut actor = self;

            let mut context = Context::new(weak_address, private_address, ActorState::Starting);

            actor.on_start(&mut context).await;

            context.state = ActorState::Started;

            loop {
                if context.state == ActorState::Stopping
                    && actor.on_stopping(&mut context).await == true
                {
                    break;
                }

                select! {
                    Some(message) = private_addr_rx.recv() => {
                        message.handle(&mut actor, &mut context).await;
                    }
                    response = addr_rx.recv() => match response {
                        Some(message) => { message.handle(&mut actor, &mut context).await },
                        None => break
                    }
                }
            }

            actor.on_stopped(&mut context).await;

            context.state = ActorState::Stopped;
        });

        address
    }

    /// Hook that runs just before the first message is processed
    async fn on_start(&mut self, _context: &mut Context<Self>) {}

    /// Hooks that runs just before the actor job is finished
    async fn on_stopped(&mut self, _context: &mut Context<Self>) {}

    /// Hook that runs if [Context::stop] method is called.
    ///
    /// Return false from this hook to prevent actor from being stopped.
    async fn on_stopping(&mut self, _context: &mut Context<Self>) -> bool {
        true
    }
}

/// `ActorSpawner` is useful when you need to store or pass somewhere and object
/// capable of spawning actors.
///
/// You may create one out of a simple closure:
/// ```
/// # use akt::{ActorSpawner, Actor};
/// # struct MyActor;
/// # impl Actor for MyActor {}
/// #
/// let spawner = ActorSpawner::from(|| MyActor);
/// ```
pub struct ActorSpawner<A: Actor> {
    spawn: Box<dyn Fn() -> A + Send>,
}

impl<A: Actor> ActorSpawner<A> {
    /// Creates ActorSpawner from a closure
    pub fn from<F: Fn() -> A + Send + 'static>(spawner: F) -> ActorSpawner<A> {
        ActorSpawner {
            spawn: Box::new(spawner),
        }
    }

    /// Spawns an actor
    pub fn spawn(&self) -> A {
        (self.spawn)()
    }

    /// Spawns an actor and immediately returns its address
    pub fn spawn_run(&self) -> Address<A> {
        (self.spawn)().run()
    }
}
