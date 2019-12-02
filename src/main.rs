use futures::prelude::*;
use std::collections::VecDeque;
use std::time::Duration;
use tokio::prelude::*;
use tokio::sync::mpsc;
use tokio::time;

struct Blackhole {}

impl Blackhole {
    fn new(rx: mpsc::UnboundedReceiver<String>, tx: mpsc::UnboundedSender<String>) -> Self {
        let mut runner = BlackholeRunner::new(rx, tx);

        tokio::spawn(async move {
            if let Err(err) = runner.entry().await {
                println!("Error: {:?}", err);
            }
        });

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

    fn entry(&mut self) -> impl Future<Output = Result<(), failure::Error>> + '_ {
        let consumer = self.consume();
        let emitter = self.emit();

        future::join(consumer, emitter).map(|_| Ok(()))
    }

    async fn consume(&mut self) {
        while let Some(object) = self.source.next().await {
            self.objects.push_back(object);
        }
    }

    async fn emit(&mut self) -> Result<(), failure::Error> {
        loop {
            time::delay_for(Duration::from_secs(2)).await;
            if let Some(object) = self.objects.pop_front() {
                let _ = self.emitter.send(object);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    let (tx, source) = mpsc::unbounded_channel();
    let (emitter, mut emission) = mpsc::unbounded_channel();

    Blackhole::new(source, emitter);

    tokio::spawn(async move {
        for i in 0..10u8 {
            time::delay_for(Duration::from_secs(1)).await;
            let _ = tx.send(format!("Body {}", i));
        }
    });

    tokio::spawn(async move {
        while let Some(object) = emission.next().await {
            println!("Emitted: {}", object);
        }
    });

    Ok(())
}
