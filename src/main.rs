use failure::_core::hash::Hasher;
use futures::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::time;

struct Blackhole {}

impl Blackhole {
    fn new(rx: mpsc::UnboundedReceiver<String>, tx: mpsc::UnboundedSender<String>) -> Self {
        let mut runner = BlackholeRunner::new(rx, tx);

        tokio::spawn(runner.entry());

        Blackhole {}
    }
}

struct BlackholeRunner {
    objects: VecDeque<String>,

    source: mpsc::UnboundedReceiver<String>,
    emitter: mpsc::UnboundedSender<String>,
}

impl BlackholeRunner {
    async fn entry(mut self) {
        self.routine().await.ok();
    }

    fn new(rx: mpsc::UnboundedReceiver<String>, tx: mpsc::UnboundedSender<String>) -> Self {
        BlackholeRunner {
            objects: VecDeque::new(),
            source: rx,
            emitter: tx,
        }
    }

    async fn routine(&mut self) -> Result<(), failure::Error> {
        let start = time::Instant::now() + time::Duration::from_secs(2);
        let mut interval = time::interval_at(start, time::Duration::from_secs(2));
        loop {
            futures::select! {
                tick = interval.tick().fuse() => {
                    if let Some(object) = self.objects.pop_front() {
                        let _ = self.emitter.send(object);
                    } else {
                        break;
                    }
                },
                object = self.source.next().fuse() => {
                    if let Some(object) = object {
                        println!("Consuming {:?}", object);
                        self.objects.push_back(object);
                    }
                },
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    let (tx, source) = mpsc::unbounded_channel();
    let (emitter, mut emission) = mpsc::unbounded_channel();

    Blackhole::new(source, emitter);

    tokio::spawn(async move {
        for i in 0..10u8 {
            time::delay_for(time::Duration::from_secs(1)).await;
            let _ = tx.send(format!("Body {}", i));
        }
    });

    while let Some(object) = emission.next().await {
        println!("Emitted: {}", object);
    }

    Ok(())
}
