use std::iter::zip;

use crate::{
    resp::{cmd, Array, BulkString, Command, FromValue, ResultValueExt, Value},
    BitmapCommands, Error, GenericCommands, GeoCommands, HashCommands, HyperLogLogCommands,
    InnerClient, ListCommands, PipelinePreparedCommand, PreparedCommand, Result, ScriptingCommands,
    ServerCommands, SetCommands, SortedSetCommands, StreamCommands, StringCommands,
};

/// Represents an on-going [`transaction`](https://redis.io/docs/manual/transactions/) on a specific client instance.
pub struct Transaction {
    client: InnerClient,
    commands: Vec<Command>,
    forget_flags: Vec<bool>,
}

impl Transaction {
    pub(crate) fn new(client: InnerClient) -> Transaction {
        let mut transaction = Transaction {
            client,
            commands: Vec::new(),
            forget_flags: Vec::new(),
        };

        transaction.queue(cmd("MULTI"));
        transaction
    }

    /// Queue a command into the transaction.
    pub fn queue(&mut self, command: Command) {
        self.commands.push(command);
        self.forget_flags.push(false);
    }

    /// Queue a command into the transaction and forget its response.
    pub fn forget(&mut self, command: Command) {
        self.commands.push(command);
        self.forget_flags.push(true);
    }

    pub async fn execute<T: FromValue>(mut self) -> Result<T> {
        self.queue(cmd("EXEC"));

        let num_commands = self.commands.len();

        let values: Vec<Value> = self.client.send_batch(self.commands).await?.into()?;
        let mut iter = values.into_iter();

        // MULTI + QUEUED commands
        for _ in 0..num_commands - 1 {
            if let Some(Value::Error(e)) = iter.next() {
                return Err(Error::Redis(e));
            }
        }

        // EXEC
        if let Some(result) = iter.next() {
            match result {
                Value::Array(Array::Vec(results)) => {
                    let mut filtered_results = zip(results, self.forget_flags.iter().skip(1))
                        .filter_map(
                            |(value, forget_flag)| if *forget_flag { None } else { Some(value) },
                        )
                        .collect::<Vec<_>>();

                    if filtered_results.len() == 1 {
                        let value = filtered_results.pop().unwrap();
                        Ok(value).into_result()?.into()
                    } else {
                        Value::Array(Array::Vec(filtered_results)).into()
                    }
                }
                Value::Array(Array::Nil) | Value::BulkString(BulkString::Nil) => Err(Error::Aborted),
                _ => Err(Error::Client("Unexpected transaction reply".to_owned())),
            }
        } else {
            Err(Error::Client(
                "Unexpected result for transaction".to_owned(),
            ))
        }
    }
}

impl<'a, R> PipelinePreparedCommand<'a, R> for PreparedCommand<'a, Transaction, R>
where
    R: FromValue + Send + 'a,
{
    /// Queue a command into the transaction.
    fn queue(self) {
        self.executor.queue(self.command)
    }

    /// Queue a command into the transaction and forget its response.
    fn forget(self) {
        self.executor.forget(self.command)
    }
}

impl BitmapCommands for Transaction {}
impl GenericCommands for Transaction {}
impl GeoCommands for Transaction {}
impl HashCommands for Transaction {}
impl HyperLogLogCommands for Transaction {}
impl ListCommands for Transaction {}
impl SetCommands for Transaction {}
impl ScriptingCommands for Transaction {}
impl ServerCommands for Transaction {}
impl SortedSetCommands for Transaction {}
impl StreamCommands for Transaction {}
impl StringCommands for Transaction {}
