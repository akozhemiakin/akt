use crate::{
    address::{UnboundedAddress, WeakAddress},
    Actor,
};

/// Context passed to each handler
pub struct Context<A: Actor> {
    address: WeakAddress<A>,
    private_address: UnboundedAddress<A>,
    pub(crate) state: ActorState,
}

impl<A: Actor> Context<A> {
    pub fn new(
        address: WeakAddress<A>,
        private_address: UnboundedAddress<A>,
        state: ActorState,
    ) -> Context<A> {
        Context {
            address,
            private_address,
            state,
        }
    }

    /// Public address
    ///
    /// Be aware that actor will not be dropped until explicitly stopped or at
    /// least one Address clone remains. Prefer to hold downgraded WeakAddress
    /// version and upgrade it when needed if possible.
    pub fn address(&self) -> WeakAddress<A> {
        self.address.clone()
    }

    /// Private address, prioritized and unbound.
    ///
    /// Should be used only by the current actor and other actors controlled by it.
    ///
    /// Unlike the main address holding private address doesn't prevent actor
    /// from stopping if all public addresses are dropped.
    pub fn private_address(&self) -> UnboundedAddress<A> {
        self.private_address.clone()
    }

    /// Stops actor gracefully
    pub fn stop(&mut self) {
        self.state = ActorState::Stopping;
    }
}

#[derive(PartialEq)]
pub enum ActorState {
    Starting,
    Started,
    Stopping,
    Stopped,
}
