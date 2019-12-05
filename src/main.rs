use failure::_core::hash::Hasher;
use futures::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::time;

struct Blackhole {}

impl Blackhole {
    fn new(rx: mpsc::UnboundedReceiver<String>, tx: mpsc::UnboundedSender<String>) -> Self {
        let runner = BlackholeRunner {
            objects: HashMap::new(),
            source: rx,
            emitter: tx,
        };

        tokio::spawn(runner.entry());

        Blackhole {}
    }
}

struct BlackholeRunner {
    objects: HashMap<String, String>,

    source: mpsc::UnboundedReceiver<String>,
    emitter: mpsc::UnboundedSender<String>,
}

impl BlackholeRunner {
    async fn entry(mut self)  {
        self.routine().await.ok();
    }

    async fn routine(&mut self) -> Result<(), failure::Error> {
        loop {
            futures::select! {
                a = self.consume().fuse() => println!("{}", a),
                b = self.emit().fuse() => println!("{}", b),
            }
        }

        Ok(())
    }

    async fn consume(&self) -> u32 {
        time::delay_for(Duration::from_secs(1)).await;
        1
    }

    async fn emit(&self) -> u32 {
        time::delay_for(Duration::from_secs(2)).await;
        2
    }

    //    async fn consume(mut source: mpsc::UnboundedReceiver<String>, objects: Deque) {
    //        while let Some(object) = source.next().await {
    //            println!("Consumed {}", object);
    //            objects.lock().await.push_back(object);
    //        }
    //    }
    //
    //    async fn emit(
    //        emitter: mpsc::UnboundedSender<String>,
    //        objects: Deque,
    //    ) -> Result<(), failure::Error> {
    //        loop {
    //            time::delay_for(Duration::from_secs(2)).await;
    //            if let Some(object) = objects.lock().await.pop_front() {
    //                let _ = emitter.send(object);
    //            } else {
    //                break;
    //            }
    //        }
    //
    //        Ok(())
    //    }
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
