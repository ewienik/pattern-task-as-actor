use {
    actor::{ActorHandle, ActorStop, MessageStop},
    ping::{Ping, PingExt},
    pong::{Pong, PongExt},
    supervisor::{self, Supervisor, SupervisorExt},
    tokio::sync::mpsc::Sender,
};

mod ping {
    use {super::*, tokio::sync::mpsc};

    pub enum Ping {
        Start { pong: Sender<Pong> },
        Ping,
        Stop,
    }

    pub trait PingExt {
        async fn start(&self, pong: Sender<Pong>);
        async fn ping(&self);
    }

    impl PingExt for Sender<Ping> {
        async fn start(&self, pong: Sender<Pong>) {
            self.send(Ping::Start { pong }).await.unwrap();
        }

        async fn ping(&self) {
            self.send(Ping::Ping).await.unwrap();
        }
    }

    impl MessageStop for Ping {
        fn message_stop() -> Self {
            Ping::Stop
        }
    }

    pub fn new(supervisor: Sender<Supervisor>) -> (Sender<Ping>, ActorHandle) {
        let (tx, mut rx) = mpsc::channel(10);
        let task = tokio::spawn({
            let tx = tx.clone();
            async move {
                println!("ping: start task");
                let mut my_pong = None;
                let mut counter = 0;
                while let Some(msg) = rx.recv().await {
                    match msg {
                        Ping::Start { pong } => {
                            my_pong.replace(pong);
                            tx.send(Ping::Ping).await.unwrap();
                        }
                        Ping::Ping => {
                            counter += 1;
                            if counter == 10 {
                                println!("done");
                                supervisor.actor_stop().await;
                            } else {
                                print!("ping..");
                                my_pong.as_ref().unwrap().pong().await;
                            }
                        }
                        Ping::Stop => {
                            rx.close();
                        }
                    }
                }
                println!("ping: finish task");
            }
        });
        (tx, task)
    }
}

mod pong {
    use {super::*, tokio::sync::mpsc};

    pub enum Pong {
        Pong,
        Stop,
    }

    pub trait PongExt {
        async fn pong(&self);
    }

    impl PongExt for Sender<Pong> {
        async fn pong(&self) {
            self.send(Pong::Pong).await.unwrap();
        }
    }

    impl MessageStop for Pong {
        fn message_stop() -> Self {
            Pong::Stop
        }
    }

    pub fn new(ping: Sender<Ping>) -> (Sender<Pong>, ActorHandle) {
        let (tx, mut rx) = mpsc::channel(10);
        let task = tokio::spawn(async move {
            println!("pong: start task");
            while let Some(msg) = rx.recv().await {
                match msg {
                    Pong::Pong => {
                        print!("pong..");
                        ping.ping().await;
                    }
                    Pong::Stop => {
                        rx.close();
                    }
                }
            }
            println!("pong: finish task");
        });
        (tx, task)
    }
}

#[cfg_attr(test, mockall::automock)]
trait Facade {
    fn new_supervisor(&self) -> (Sender<Supervisor>, ActorHandle) {
        supervisor::new()
    }
    fn new_ping(&self, supervisor: Sender<Supervisor>) -> (Sender<Ping>, ActorHandle) {
        ping::new(supervisor)
    }
    fn new_pong(&self, ping: Sender<Ping>) -> (Sender<Pong>, ActorHandle) {
        pong::new(ping)
    }
}

struct FacadeDefault;

impl Facade for FacadeDefault {
    fn new_supervisor(&self) -> (Sender<Supervisor>, ActorHandle) {
        supervisor::new()
    }
    fn new_ping(&self, supervisor: Sender<Supervisor>) -> (Sender<Ping>, ActorHandle) {
        ping::new(supervisor)
    }
    fn new_pong(&self, ping: Sender<Ping>) -> (Sender<Pong>, ActorHandle) {
        pong::new(ping)
    }
}

#[tokio::main]
async fn main() {
    run(FacadeDefault).await;
}

async fn run(facade: impl Facade) {
    let (supervisor, supervisor_task) = facade.new_supervisor();
    let (ping, ping_task) = facade.new_ping(supervisor.clone());
    supervisor.attach(ping.clone(), ping_task).await;
    let (pong, pong_task) = facade.new_pong(ping.clone());
    supervisor.attach(pong.clone(), pong_task).await;
    ping.start(pong).await;
    supervisor_task.await.unwrap();
}

#[cfg(test)]
mod tests {
    use {super::*, tokio::sync::mpsc};

    #[tokio::test]
    async fn process_run() {
        let mut facade = MockFacade::new();
        let (tx_supervisor, mut rx_supervisor) = mpsc::channel(10);
        let (tx_ping, mut rx_ping) = mpsc::channel(10);
        let (tx_pong, _) = mpsc::channel(10);
        facade
            .expect_new_supervisor()
            .once()
            .return_once(|| (tx_supervisor, tokio::spawn(async {})));
        facade
            .expect_new_ping()
            .once()
            .return_once(|_| (tx_ping, tokio::spawn(async {})));
        facade
            .expect_new_pong()
            .once()
            .return_once(|_| (tx_pong, tokio::spawn(async {})));
        let task = tokio::spawn(run(facade));
        assert!(matches!(
            rx_supervisor.recv().await.unwrap(),
            Supervisor::Attach { .. }
        ));
        assert!(matches!(
            rx_supervisor.recv().await.unwrap(),
            Supervisor::Attach { .. }
        ));
        assert!(matches!(rx_ping.recv().await.unwrap(), Ping::Start { .. }));
        task.await.unwrap();
    }
}
