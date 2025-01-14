use {
    std::future::Future,
    tokio::{sync::mpsc::Sender, task::JoinHandle},
};

pub type ActorHandle = JoinHandle<()>;

pub trait ActorStop {
    fn actor_stop(&self) -> impl Future<Output = ()> + Send;
}

impl<T: MessageStop> ActorStop for Sender<T> {
    async fn actor_stop(&self) {
        _ = self.send(T::message_stop()).await;
    }
}

pub trait MessageStop: Send {
    fn message_stop() -> Self;
}
