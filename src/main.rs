use futures::prelude::*;
use std::collections::VecDeque;
use tokio::sync::mpsc;
use tokio::time::{delay_for, interval_at, Duration, Instant};

struct Blackhole {}

impl Blackhole {
    fn new(rx: mpsc::UnboundedReceiver<String>, tx: mpsc::UnboundedSender<String>) -> Self {
        let runner = BlackholeRunner::new(rx, tx);

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
    fn new(rx: mpsc::UnboundedReceiver<String>, tx: mpsc::UnboundedSender<String>) -> Self {
        BlackholeRunner {
            objects: VecDeque::new(),
            source: rx,
            emitter: tx,
        }
    }

    async fn entry(mut self) {
        self.routine().await.ok();
    }

    async fn routine(&mut self) -> Result<(), failure::Error> {
        let start = Instant::now() + Duration::from_secs(2);
        let mut interval = interval_at(start, Duration::from_secs(2));
        loop {
            futures::select! {
                tick = interval.tick().fuse() => {
                    if !self.emit() { break; }
                },
                object = self.source.next().fuse() => {
                    if let Some(object) = object {
                        self.consume(object);
                    }
                },
            }
        }

        Ok(())
    }

    fn emit(&mut self) -> bool {
        if let Some(object) = self.objects.pop_front() {
            let _ = self.emitter.send(object);
            return true;
        }
        false
    }

    fn consume(&mut self, object: String) {
        println!("Consuming: {}", object);
        self.objects.push_back(object);
    }
}

#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    let (tx, source) = mpsc::unbounded_channel();
    let (emitter, mut emission) = mpsc::unbounded_channel();

    Blackhole::new(source, emitter);

    tokio::spawn(async move {
        for i in 0..10u8 {
            delay_for(Duration::from_secs(1)).await;
            let _ = tx.send(format!("Body {}", i));
        }
    });

    while let Some(object) = emission.next().await {
        println!("Emitted: {}", object);
    }

    Ok(())
}
