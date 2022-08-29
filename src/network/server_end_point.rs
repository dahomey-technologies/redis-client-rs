use crate::{Command, Connection, InteractiveConnection, Message, PubSubConnection, Result, ConnectionFactory};

#[derive(Debug, Copy, Clone)]
pub(crate) enum ConnectionType {
    Interactive,
    PubSub,
}

#[derive(Clone)]
pub(crate) struct ServerEndPoint {
    connection_factory: ConnectionFactory,
    interactive: InteractiveConnection,
    pubsub: PubSubConnection,
}

impl ServerEndPoint {
    pub async fn connect(addr: impl Into<String>) -> Result<Self> {
        let connection_factory = ConnectionFactory::initialize(addr).await?;
        let interactive = InteractiveConnection::connect(&connection_factory).await?;
        let pubsub = PubSubConnection::connect(&connection_factory).await?;

        Ok(Self {
            connection_factory,
            interactive,
            pubsub,
        })
    }

    pub fn get_addr(&self) -> &str {
        self.connection_factory.get_addr()
    }

    fn get_connection(&self, command: &Command) -> &dyn Connection {
        match command.name {
            "SUBSCRIBE" => &self.pubsub,
            "UNSUBSCRIBE" => &self.pubsub,
            "PSUBSCRIBE" => &self.pubsub,
            "PUNSUBSCRIBE" => &self.pubsub,
            _ => &self.interactive,
        }
    }

    pub fn send(&self, message: Message) -> Result<()> {
        let connection = self.get_connection(&message.command);
        connection.send(message)?;
        Ok(())
    }
}
