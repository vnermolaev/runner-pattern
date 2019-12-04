use futures::prelude::*;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::time;

struct Blackhole {}

impl Blackhole {
    fn new(rx: mpsc::UnboundedReceiver<String>, tx: mpsc::UnboundedSender<String>) -> Self {
        let runner = BlackholeRunner::new(rx, tx);

        tokio::spawn(async move {
            if let Err(err) = runner.entry().await {
                println!("Error: {:?}", err);
            }
        });

        Blackhole {}
    }
}

type Deque = Arc<Mutex<VecDeque<String>>>;

struct BlackholeRunner {
    objects: Deque,

    source: mpsc::UnboundedReceiver<String>,
    emitter: mpsc::UnboundedSender<String>,
}

impl BlackholeRunner {
    fn new(rx: mpsc::UnboundedReceiver<String>, tx: mpsc::UnboundedSender<String>) -> Self {
        BlackholeRunner {
            objects: Arc::new(Mutex::new(VecDeque::new())),
            source: rx,
            emitter: tx,
        }
    }

    fn entry(self) -> impl Future<Output = Result<(), failure::Error>> {
        let Self {
            objects,
            source,
            emitter,
        } = self;
        let consumer = BlackholeRunner::consume(source, objects.clone());
        let emitter = BlackholeRunner::emit(emitter, objects.clone());

        future::join(consumer, emitter).map(|_| Ok(()))
    }

    async fn consume(mut source: mpsc::UnboundedReceiver<String>, objects: Deque) {
        while let Some(object) = source.next().await {
            println!("Consumed {}", object);
            objects.lock().await.push_back(object);
        }
    }

    async fn emit(
        emitter: mpsc::UnboundedSender<String>,
        objects: Deque,
    ) -> Result<(), failure::Error> {
        loop {
            time::delay_for(Duration::from_secs(2)).await;
            if let Some(object) = objects.lock().await.pop_front() {
                let _ = emitter.send(object);
            } else {
                break;
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
            time::delay_for(Duration::from_secs(1)).await;
            let _ = tx.send(format!("Body {}", i));
        }
    });

    while let Some(object) = emission.next().await {
        println!("Emitted: {}", object);
    }

    Ok(())
}
