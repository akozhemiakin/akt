//! An actors framework for Rust and Tokio.
//!
//! It is heavily inspired by Actix and right now it has very similar look
//! and feel. The main difference is that in Actix (at least at the moment
//! of this particular library creation) working with async / await and async
//! handlers in particular is a little cumbersome. This library supports async
//! handlers out of the box and even provide tools to simplify some unusual
//! kinds of communications like waiting for result without event blocking
//! main message loop.
//!
//! Let's implement some actor to manage bank account.
//! ```
//! use akt::{Actor, Context, Handler, Message};
//! use async_trait::async_trait;
//! use thiserror::Error;
//!
//! // A state managed by our actor
//! struct Account {
//!     balance: u32,
//! }
//!
//! // Implementing Actor trait makes an actor out of the state
//! impl Actor for Account {}
//!
//! // Define messages to be handled by the actor
//!
//! struct Deposit {
//!     pub amount: u32,
//! }
//!
//! impl Message for Deposit {
//!   // Result the actor will respond with
//!     type Result = u32;
//! }
//!
//! // Implement handler for the Deposit message and Account actor
//! // We need to use `async_trait` crate "magic" because async trait methods
//! // is not supported bu rust directly yet.
//! #[async_trait]
//! impl Handler<Deposit> for Account {
//!     async fn handle(&mut self, message: Deposit, _context: &mut Context<Account>) -> u32 {
//!         self.balance = self.balance + message.amount;
//!
//!         self.balance
//!     }
//! }
//!
//! struct Withdraw {
//!   pub amount: u32,
//! }
//!
//! // Withdrawal may fail so we could provide specific error for it.
//! //
//! // It could be a struct, but for backwards compatibility reasons it could
//! // be a good idea to make enum even if for now we have only one variant.
//! //
//! // Here we use `thiserror` crate for convenient Error
//! // derivation, but it is completely optional.
//! #[derive(Debug, Error, PartialEq)]
//! enum WithdrawalError {
//!     #[error("Insufficient funds. {requested} was requested but only {available} is available.")]
//!     InsufficientFunds { requested: u32, available: u32 },
//! }
//!
//! impl Message for Withdraw {
//!     type Result = Result<u32, WithdrawalError>;
//! }
//!
//! #[async_trait]
//! impl Handler<Withdraw> for Account {
//!     async fn handle(
//!         &mut self,
//!         message: Withdraw,
//!         _context: &mut Context<Account>,
//!     ) -> Result<u32, WithdrawalError> {
//!         if self.balance < message.amount {
//!             return Err(WithdrawalError::InsufficientFunds {
//!                 requested: message.amount,
//!                 available: self.balance,
//!             });
//!         }
//!
//!         self.balance = self.balance - message.amount;
//!
//!         Ok(self.balance)
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!    // Create actor
//!    let actor = Account { balance: 50 };
//!
//!    // Run actor consuming it and returning its address
//!    let address = actor.run();
//!
//!    // Send message and use regular `await` workflow to wait for the response.
//!    let balance = address.send(Withdraw { amount: 20 }).await;
//!    assert_eq!(balance, Ok(Ok(30)));
//!
//!    let balance = address.send(Deposit { amount: 40 }).await;
//!    assert_eq!(balance, Ok(70));
//!
//!    let balance = address.send(Withdraw { amount: 100 }).await;
//!    assert_eq!(
//!        balance,
//!        Ok(Err(WithdrawalError::InsufficientFunds {
//!            requested: 100,
//!            available: 70
//!        }))
//!    );
//!
//!    // Actors addresses could be safely cloned
//!    let address_clone = address.clone();
//!    let balance = address_clone.send(Withdraw { amount: 10 }).await;
//!    assert_eq!(balance, Ok(Ok(60)));
//!
//!    let balance = address.send(Withdraw { amount: 10 }).await;
//!    assert_eq!(balance, Ok(Ok(50)));
//! }
//! ```

mod actor;
mod address;
mod context;
mod handler;

pub use self::{
    actor::{Actor, ActorSpawner},
    address::{ActorSendError, Address, FailedToDeliver, Message, UnboundedAddress},
    context::{ActorState, Context},
    handler::Handler,
};

#[cfg(feature = "error-stack")]
mod error_stack;
#[cfg(feature = "error-stack")]
pub use self::error_stack::ActorResultIntoReport;
