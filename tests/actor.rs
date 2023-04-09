use std::sync::atomic::{AtomicBool, Ordering};

use akt::{Actor, Context};
use async_trait::async_trait;
use tokio::task::yield_now;

struct DroppingActor {
    is_stopped: &'static AtomicBool,
}

#[async_trait]
impl Actor for DroppingActor {
    async fn on_stopped(&mut self, _context: &mut Context<DroppingActor>) {
        self.is_stopped.store(true, Ordering::Release);
    }
}

static IS_STOPPED_CALLED: AtomicBool = AtomicBool::new(false);

#[tokio::test]
async fn stops_on_no_addresses_left() {
    let actor = DroppingActor {
        is_stopped: &IS_STOPPED_CALLED,
    }
    .run();

    drop(actor);

    yield_now().await;

    assert_eq!(IS_STOPPED_CALLED.load(Ordering::Acquire), true);
}
