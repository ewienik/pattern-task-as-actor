use {
    futures::StreamExt,
    tokio::{
        sync::{mpsc, oneshot},
        task::JoinHandle,
    },
    tokio_stream::wrappers::ReceiverStream,
};

enum Message {
    One { tx: oneshot::Sender<String> },
    Two,
}

#[async_trait::async_trait]
trait MessageExt {
    async fn one(&self) -> String;
    async fn two(&self);
}

#[async_trait::async_trait]
impl MessageExt for mpsc::Sender<Message> {
    async fn one(&self) -> String {
        let (tx, rx) = oneshot::channel();
        self.send(Message::One { tx }).await.unwrap();
        rx.await.unwrap()
    }
    async fn two(&self) {
        self.send(Message::Two).await.unwrap();
    }
}

fn new_task() -> (mpsc::Sender<Message>, JoinHandle<()>) {
    let (tx, rx) = mpsc::channel(10);
    let task = tokio::spawn(async move {
        println!("Started task");
        ReceiverStream::new(rx)
            .for_each(|msg| async move {
                match msg {
                    Message::One { tx } => {
                        println!("Received one!");
                        tx.send("Hello!".to_string()).unwrap();
                    }
                    Message::Two => {
                        println!("Received two!");
                    }
                }
            })
            .await;
        println!("Finished task");
    });
    (tx, task)
}

#[tokio::main]
async fn main() {
    let (tx, task) = new_task();

    println!("Received: {}", tx.one().await);
    tx.two().await;
    println!("Received: {}", tx.one().await);
    tx.two().await;
    drop(tx);
    task.await.unwrap();
}

#[cfg(test)]
mod tests {}
