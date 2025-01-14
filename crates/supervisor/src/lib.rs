use {
    actor::{ActorHandle, ActorStop, MessageStop},
    futures::{
        self,
        future::{self, BoxFuture},
        FutureExt,
    },
    std::future::Future,
    tokio::sync::mpsc::{self, Sender},
};

pub enum Supervisor {
    Attach {
        stop: BoxFuture<'static, ()>,
        handle: ActorHandle,
    },
    Stop,
}

impl MessageStop for Supervisor {
    fn message_stop() -> Self {
        Supervisor::Stop
    }
}

pub trait SupervisorExt {
    fn attach(
        &self,
        actor: impl ActorStop + Send + 'static,
        handle: ActorHandle,
    ) -> impl Future<Output = ()>;
}

impl SupervisorExt for Sender<Supervisor> {
    async fn attach(&self, actor: impl ActorStop + Send + 'static, handle: ActorHandle) {
        self.send(Supervisor::Attach {
            stop: async move {
                actor.actor_stop().await;
            }
            .boxed(),
            handle,
        })
        .await
        .unwrap();
    }
}

pub fn new() -> (Sender<Supervisor>, ActorHandle) {
    let (tx, mut rx) = mpsc::channel(10);
    let task = tokio::spawn(async move {
        println!("supervisor: start task");
        let mut stops = Vec::new();
        let mut handles = Vec::new();
        while let Some(msg) = rx.recv().await {
            match msg {
                Supervisor::Attach { stop, handle } => {
                    stops.push(stop);
                    handles.push(handle);
                }
                Supervisor::Stop => rx.close(),
            }
        }
        future::join_all(stops.into_iter()).await;
        future::join_all(handles.into_iter()).await;
        println!("supervisor: finish task");
    });
    (tx, task)
}
