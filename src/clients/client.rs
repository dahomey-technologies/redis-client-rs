use crate::{
    network::{MonitorReceiver, MonitorSender},
    resp::{cmd, BulkString, Command, FromValue, ResultValueExt, SingleArgOrCollection, Value},
    BitmapCommands, BlockingCommands, ClusterCommands, ConnectionCommands, Future, GenericCommands,
    GeoCommands, HashCommands, HyperLogLogCommands, InnerClient, InternalPubSubCommands,
    IntoConfig, ListCommands, Message, MonitorStream, Pipeline, PreparedCommand, PubSubCommands,
    PubSubStream, Result, ScriptingCommands, SentinelCommands, ServerCommands, SetCommands,
    SortedSetCommands, StreamCommands, StringCommands, Transaction, TransactionCommands,
    ValueReceiver, ValueSender,
};
use futures::channel::{mpsc, oneshot};
use std::future::IntoFuture;

/// Client with a unique connection to a Redis server.
pub struct Client {
    inner_client: InnerClient,
}

impl Client {
    /// Connects asynchronously to the Redis server.
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the connection operation
    pub async fn connect(config: impl IntoConfig) -> Result<Self> {
        let inner_client = InnerClient::connect(config).await?;
        Ok(Self { inner_client })
    }

    /// We don't want the Client struct to be publicly cloneable
    /// If one wants to consume a multiplexed client,
    /// the [MultiplexedClient](crate::MultiplexedClient) must be used instead
    pub(crate) fn clone(&self) -> Client {
        Client {
            inner_client: self.inner_client.clone(),
        }
    }

    /// Send an arbitrary command to the server.
    ///
    /// This is used primarily intended for implementing high level commands API
    /// but may also be used to provide access to new features that lack a direct API.
    ///
    /// # Arguments
    /// * `name` - Command name in uppercase.
    /// * `args` - Command arguments which can be provided as arrays (up to 4 elements) or vectors of [`BulkString`](crate::resp::BulkString).
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the send operation
    ///
    /// # Example
    /// ```
    /// use redis_driver::{resp::cmd, Client, Result};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let mut client = Client::connect("127.0.0.1:6379").await?;
    ///
    ///     let values: Vec<String> = client
    ///         .send(cmd("MGET").arg("key1").arg("key2").arg("key3").arg("key4"))
    ///         .await?
    ///         .into()?;
    ///     println!("{:?}", values);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn send(&mut self, command: Command) -> Result<Value> {
        self.inner_client.send(command).await
    }

    /// Send command to the Redis server and forget its response.
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the send operation
    pub fn send_and_forget(&mut self, command: Command) -> Result<()> {
        self.inner_client.send_and_forget(command)
    }

    /// Send a command batch to the Redis server.
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the send operation
    pub async fn send_batch(&mut self, commands: Vec<Command>) -> Result<Value> {
        self.inner_client.send_batch(commands).await
    }

    /// Create a new transaction
    pub fn create_transaction(&mut self) -> Transaction {
        Transaction::new(self.inner_client.clone())
    }

    /// Create a new pipeline
    pub fn create_pipeline(&mut self) -> Pipeline {
        Pipeline::new(self.inner_client.clone())
    }
}

pub trait ClientPreparedCommand<'a, R>
where
    R: FromValue,
{
    /// Send command and forget its response
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occur during the send operation
    fn forget(self) -> Result<()>;
}

impl<'a, R> ClientPreparedCommand<'a, R> for PreparedCommand<'a, Client, R>
where
    R: FromValue + Send + 'a,
{
    /// Send command and forget its response
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occur during the send operation
    fn forget(self) -> Result<()> {
        self.executor.send_and_forget(self.command)
    }
}

impl<'a, R> IntoFuture for PreparedCommand<'a, Client, R>
where
    R: FromValue + Send + 'a,
{
    type Output = Result<R>;
    type IntoFuture = Future<'a, R>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.executor.send(self.command).await?.into() })
    }
}

impl BitmapCommands for Client {}
impl ClusterCommands for Client {}
impl ConnectionCommands for Client {}
impl GenericCommands for Client {}
impl GeoCommands for Client {}
impl HashCommands for Client {}
impl HyperLogLogCommands for Client {}
impl InternalPubSubCommands for Client {}
impl ListCommands for Client {}
impl ScriptingCommands for Client {}
impl SentinelCommands for Client {}
impl ServerCommands for Client {}
impl SetCommands for Client {}
impl SortedSetCommands for Client {}
impl StreamCommands for Client {}
impl StringCommands for Client {}
impl TransactionCommands for Client {}

impl PubSubCommands for Client {
    fn subscribe<'a, C, CC>(&'a mut self, channels: CC) -> Future<'a, PubSubStream>
    where
        C: Into<BulkString> + Send + 'a,
        CC: SingleArgOrCollection<C>,
    {
        self.inner_client.subscribe(channels)
    }

    fn psubscribe<'a, P, PP>(&'a mut self, patterns: PP) -> Future<'a, PubSubStream>
    where
        P: Into<BulkString> + Send + 'a,
        PP: SingleArgOrCollection<P>,
    {
        self.inner_client.psubscribe(patterns)
    }
}

impl BlockingCommands for Client {
    fn monitor(&mut self) -> Future<crate::MonitorStream> {
        Box::pin(async move {
            let (value_sender, value_receiver): (ValueSender, ValueReceiver) = oneshot::channel();
            let (monitor_sender, monitor_receiver): (MonitorSender, MonitorReceiver) =
                mpsc::unbounded();

            let message = Message::monitor(cmd("MONITOR"), value_sender, monitor_sender);

            self.inner_client.send_message(message)?;

            let value = value_receiver.await?;
            value.map_into_result(|_| MonitorStream::new(monitor_receiver, self.clone()))
        })
    }
}
