#[cfg(feature = "redis-graph")]
use crate::commands::GraphCommands;
#[cfg(feature = "redis-json")]
use crate::commands::JsonCommands;
#[cfg(feature = "redis-search")]
use crate::commands::SearchCommands;
#[cfg(feature = "redis-time-series")]
use crate::commands::TimeSeriesCommands;
#[cfg(feature = "redis-bloom")]
use crate::commands::{
    BloomCommands, CountMinSketchCommands, CuckooCommands, TDigestCommands, TopKCommands,
};
use crate::{
    client::{Client, PreparedCommand},
    commands::{
        BitmapCommands, ClusterCommands, ConnectionCommands, GenericCommands, GeoCommands,
        HashCommands, HyperLogLogCommands, ListCommands, ScriptingCommands, ServerCommands,
        SetCommands, SortedSetCommands, StreamCommands, StringCommands,
    },
    resp::{Command, RespBatchDeserializer, Response},
    Result,
};
use serde::de::DeserializeOwned;
use std::iter::zip;

/// Represents a Redis command pipeline.
pub struct Pipeline {
    client: Client,
    commands: Vec<Command>,
    forget_flags: Vec<bool>,
    retry_on_error: Option<bool>,
}

impl Pipeline {
    pub(crate) fn new(client: Client) -> Pipeline {
        Pipeline {
            client,
            commands: Vec::new(),
            forget_flags: Vec::new(),
            retry_on_error: None,
        }
    }
    /// Set a flag to override default `retry_on_error` behavior.
    ///
    /// See [Config::retry_on_error](crate::client::Config::retry_on_error)
    pub fn retry_on_error(&mut self, retry_on_error: bool) {
        self.retry_on_error = Some(retry_on_error);
    }

    /// Queue a command
    pub fn queue(&mut self, command: Command) {
        self.commands.push(command);
        self.forget_flags.push(false);
    }

    /// Queue a command and forget its response
    pub fn forget(&mut self, command: Command) {
        self.commands.push(command);
        self.forget_flags.push(true);
    }

    /// Execute the pipeline by the sending the queued command
    /// as a whole batch to the Redis server.
    ///
    /// # Return
    /// It is the caller responsability to use the right type to cast the server response
    /// to the right tuple or collection depending on which command has been
    /// [queued](BatchPreparedCommand::queue) or [forgotten](BatchPreparedCommand::forget).
    ///
    /// The most generic type that can be requested as a result is `Vec<resp::Value>`
    ///
    /// # Example
    /// ```
    /// use rustis::{
    ///     client::{Client, Pipeline, BatchPreparedCommand},
    ///     commands::StringCommands,
    ///     resp::{cmd, Value}, Result,
    /// };
    ///
    /// #[cfg_attr(feature = "tokio-runtime", tokio::main)]
    /// #[cfg_attr(feature = "async-std-runtime", async_std::main)]
    /// async fn main() -> Result<()> {
    ///     let mut client = Client::connect("127.0.0.1:6379").await?;
    ///
    ///     let mut pipeline = client.create_pipeline();
    ///     pipeline.set("key1", "value1").forget();
    ///     pipeline.set("key2", "value2").forget();
    ///     pipeline.get::<_, String>("key1").queue();
    ///     pipeline.get::<_, String>("key2").queue();
    ///
    ///     let (value1, value2): (String, String) = pipeline.execute().await?;
    ///     assert_eq!("value1", value1);
    ///     assert_eq!("value2", value2);
    ///
    ///     Ok(())
    /// }
    /// ```    
    pub async fn execute<T: DeserializeOwned>(mut self) -> Result<T> {
        let num_commands = self.commands.len();
        let results = self
            .client
            .send_batch(self.commands, self.retry_on_error)
            .await?;

        if num_commands > 1 {
            let mut filtered_results = zip(results, self.forget_flags.iter())
                .filter_map(|(value, forget_flag)| if *forget_flag { None } else { Some(value) })
                .collect::<Vec<_>>();

            if filtered_results.len() == 1 {
                let result = filtered_results.pop().unwrap();
                result.to()
            } else {
                let deserializer = RespBatchDeserializer::new(&filtered_results);
                T::deserialize(&deserializer)
            }
        } else {
            results[0].to()
        }
    }
}

/// Extension trait dedicated to [`PreparedCommand`](crate::client::PreparedCommand)
/// to add specific methods for the [`Pipeline`](crate::client::Pipeline) &
/// the [`Transaction`](crate::client::Transaction) executors
pub trait BatchPreparedCommand<R = ()> {
    /// Queue a command.
    fn queue(self);

    /// Queue a command and forget its response.
    fn forget(self);
}

impl<R: Response> BatchPreparedCommand for PreparedCommand<'_, Pipeline, R> {
    /// Queue a command.
    #[inline]
    fn queue(self) {
        self.executor.queue(self.command)
    }

    /// Queue a command and forget its response.
    #[inline]
    fn forget(self) {
        self.executor.forget(self.command)
    }
}

impl BitmapCommands for Pipeline {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl BloomCommands for Pipeline {}
impl ClusterCommands for Pipeline {}
impl ConnectionCommands for Pipeline {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl CountMinSketchCommands for Pipeline {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl CuckooCommands for Pipeline {}
impl GenericCommands for Pipeline {}
impl GeoCommands for Pipeline {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-graph")))]
#[cfg(feature = "redis-graph")]
impl GraphCommands for Pipeline {}
impl HashCommands for Pipeline {}
impl HyperLogLogCommands for Pipeline {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-json")))]
#[cfg(feature = "redis-json")]
impl JsonCommands for Pipeline {}
impl ListCommands for Pipeline {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-search")))]
#[cfg(feature = "redis-search")]
impl SearchCommands for Pipeline {}
impl SetCommands for Pipeline {}
impl ScriptingCommands for Pipeline {}
impl ServerCommands for Pipeline {}
impl SortedSetCommands for Pipeline {}
impl StreamCommands for Pipeline {}
impl StringCommands for Pipeline {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl TDigestCommands for Pipeline {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-time-series")))]
#[cfg(feature = "redis-time-series")]
impl TimeSeriesCommands for Pipeline {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl TopKCommands for Pipeline {}
