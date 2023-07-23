use {ping::Ping, pong::Pong, supervisor::Supervisor, tokio::task::JoinHandle};

pub type ActorHandle = JoinHandle<()>;

#[async_trait::async_trait]
pub trait Stop {
    async fn stop(&self);
}

mod supervisor {
    use {
        super::*,
        futures::{future, stream, StreamExt},
        tokio::sync::mpsc::{self, Sender},
    };

    enum Message {
        Attach {
            actor: Box<dyn Stop + Send + Sync>,
            handle: ActorHandle,
        },
        Stop,
    }

    #[derive(Clone)]
    pub struct Supervisor(Sender<Message>);

    impl Supervisor {
        pub async fn attach(&self, actor: Box<dyn Stop + Send + Sync>, handle: ActorHandle) {
            self.0
                .send(Message::Attach { actor, handle })
                .await
                .unwrap();
        }

        pub fn new() -> (Supervisor, ActorHandle) {
            let (tx, mut rx) = mpsc::channel(10);
            let task = tokio::spawn(async move {
                println!("supervisor: start task");
                let mut actors = Vec::new();
                let mut handles = Vec::new();
                while let Some(msg) = rx.recv().await {
                    match msg {
                        Message::Attach { actor, handle } => {
                            actors.push(actor);
                            handles.push(handle);
                        }
                        Message::Stop => rx.close(),
                    }
                }
                stream::iter(actors.into_iter())
                    .for_each(|actor| async move {
                        actor.stop().await;
                    })
                    .await;
                future::join_all(handles.into_iter()).await;
                println!("supervisor: finish task");
            });
            (Supervisor(tx), task)
        }
    }

    #[async_trait::async_trait]
    impl Stop for Supervisor {
        async fn stop(&self) {
            self.0.send(Message::Stop).await.unwrap();
        }
    }
}

mod ping {
    use {
        super::*,
        tokio::sync::mpsc::{self, Sender},
    };

    enum Message {
        Start { pong: Pong },
        Ping,
        Stop,
    }

    #[derive(Clone)]
    pub struct Ping(Sender<Message>);

    impl Ping {
        pub async fn start(&self, pong: Pong) {
            self.0.send(Message::Start { pong }).await.unwrap();
        }

        pub async fn ping(&self) {
            self.0.send(Message::Ping).await.unwrap();
        }

        pub fn new(supervisor: Supervisor) -> (Ping, ActorHandle) {
            let (tx, mut rx) = mpsc::channel(10);
            let task = tokio::spawn({
                let tx = tx.clone();
                async move {
                    println!("ping: start task");
                    let mut my_pong = None;
                    let mut counter = 0;
                    while let Some(msg) = rx.recv().await {
                        match msg {
                            Message::Start { pong } => {
                                my_pong.replace(pong);
                                tx.send(Message::Ping).await.unwrap();
                            }
                            Message::Ping => {
                                counter += 1;
                                if counter == 10 {
                                    println!("done");
                                    supervisor.stop().await;
                                } else {
                                    print!("ping..");
                                    my_pong.as_ref().unwrap().pong().await;
                                }
                            }
                            Message::Stop => {
                                rx.close();
                            }
                        }
                    }
                    println!("ping: finish task");
                }
            });
            (Ping(tx), task)
        }
    }

    #[async_trait::async_trait]
    impl Stop for Ping {
        async fn stop(&self) {
            self.0.send(Message::Stop).await.unwrap();
        }
    }
}

mod pong {
    use {
        super::*,
        tokio::sync::mpsc::{self, Sender},
    };

    enum Message {
        Pong,
        Stop,
    }

    #[derive(Clone)]
    pub struct Pong(Sender<Message>);

    impl Pong {
        pub async fn pong(&self) {
            self.0.send(Message::Pong).await.unwrap();
        }

        pub fn new(ping: Ping) -> (Pong, ActorHandle) {
            let (tx, mut rx) = mpsc::channel(10);
            let task = tokio::spawn(async move {
                println!("pong: start task");
                while let Some(msg) = rx.recv().await {
                    match msg {
                        Message::Pong => {
                            print!("pong..");
                            ping.ping().await;
                        }
                        Message::Stop => {
                            rx.close();
                        }
                    }
                }
                println!("pong: finish task");
            });
            (Pong(tx), task)
        }
    }

    #[async_trait::async_trait]
    impl Stop for Pong {
        async fn stop(&self) {
            self.0.send(Message::Stop).await.unwrap();
        }
    }
}

#[tokio::main]
async fn main() {
    let (supervisor, supervisor_task) = Supervisor::new();
    let (ping, ping_task) = Ping::new(supervisor.clone());
    supervisor.attach(Box::new(ping.clone()), ping_task).await;
    let (pong, pong_task) = Pong::new(ping.clone());
    supervisor.attach(Box::new(pong.clone()), pong_task).await;
    ping.start(pong).await;
    supervisor_task.await.unwrap();
}

#[cfg(test)]
mod tests {}
