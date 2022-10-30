use std::iter::zip;

use crate::{
    resp::{Array, Command, FromValue, Value, ResultValueExt},
    BitmapCommands, ConnectionCommands, GenericCommands, GeoCommands, HashCommands,
    HyperLogLogCommands, ListCommands, PreparedCommand, Result, ScriptingCommands, ServerCommands,
    SetCommands, SortedSetCommands, StreamCommands, StringCommands, Error, InnerClient,
};

pub struct Pipeline {
    client: InnerClient,
    commands: Vec<Command>,
    forget_flags: Vec<bool>,
}

impl Pipeline {
    pub(crate) fn new(client: InnerClient) -> Pipeline {
        Pipeline {
            client,
            commands: Vec::new(),
            forget_flags: Vec::new(),
        }
    }

    /// Queue a command
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occur during the send operation
    pub fn queue(&mut self, command: Command) {
        self.commands.push(command);
        self.forget_flags.push(false);
    }

    /// Queue a command and forget its response
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occur during the send operation
    pub fn forget(&mut self, command: Command) {
        self.commands.push(command);
        self.forget_flags.push(true);
    }

    pub async fn execute<T: FromValue>(mut self) -> Result<T> {
        let result = self.client.send_batch(self.commands).await?;

        match result {
            Value::Array(Array::Vec(results)) => {
                let mut filtered_results = zip(results, self.forget_flags.iter())
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
            _ => Err(Error::Client("Unexpected pipeline reply".to_owned())),
        }
    }
}

pub trait PipelinePreparedCommand<'a, R>
where
    R: FromValue,
{
    /// Queue a command and forget its response
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occur during the send operation
    fn queue(self);

    /// Queue a command and forget its response
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occur during the send operation
    fn forget(self);
}

impl<'a, R> PipelinePreparedCommand<'a, R> for PreparedCommand<'a, Pipeline, R>
where
    R: FromValue + Send + 'a,
{
    /// Queue a command and forget its response
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occur during the send operation
    fn queue(self) {
        self.executor.queue(self.command)
    }

    /// Queue a command and forget its response
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occur during the send operation
    fn forget(self) {
        self.executor.forget(self.command)
    }
}

impl BitmapCommands for Pipeline {}
impl ConnectionCommands for Pipeline {}
impl GenericCommands for Pipeline {}
impl GeoCommands for Pipeline {}
impl HashCommands for Pipeline {}
impl HyperLogLogCommands for Pipeline {}
impl ListCommands for Pipeline {}
impl SetCommands for Pipeline {}
impl ScriptingCommands for Pipeline {}
impl ServerCommands for Pipeline {}
impl SortedSetCommands for Pipeline {}
impl StreamCommands for Pipeline {}
impl StringCommands for Pipeline {}
