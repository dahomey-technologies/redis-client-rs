use crate::{
    client::{Config, PreparedCommand},
    commands::{ClusterCommands, ConnectionCommands, SentinelCommands, ServerCommands, HelloOptions},
    resp::{Command, CommandEncoder, FromValue, ResultValueExt, Value, ValueDecoder},
    tcp_connect, Error, Future, Result, RetryReason, TcpStreamReader, TcpStreamWriter,
};
#[cfg(feature = "tls")]
use crate::{tcp_tls_connect, TcpTlsStreamReader, TcpTlsStreamWriter};
use bytes::BytesMut;
use futures::{SinkExt, StreamExt};
use log::{debug, log_enabled, Level};
use std::future::IntoFuture;
use tokio::io::AsyncWriteExt;
use tokio_util::codec::{Encoder, FramedRead, FramedWrite};

pub(crate) enum Streams {
    Tcp(
        FramedRead<TcpStreamReader, ValueDecoder>,
        FramedWrite<TcpStreamWriter, CommandEncoder>,
    ),
    #[cfg(feature = "tls")]
    TcpTls(
        FramedRead<TcpTlsStreamReader, ValueDecoder>,
        FramedWrite<TcpTlsStreamWriter, CommandEncoder>,
    ),
}

impl Streams {
    pub async fn connect(host: &str, port: u16, config: &Config) -> Result<Self> {
        #[cfg(feature = "tls")]
        if let Some(tls_config) = &config.tls_config {
            let (reader, writer) =
                tcp_tls_connect(host, port, tls_config, config.connect_timeout).await?;
            let framed_read = FramedRead::new(reader, ValueDecoder);
            let framed_write = FramedWrite::new(writer, CommandEncoder);
            Ok(Streams::TcpTls(framed_read, framed_write))
        } else {
            Self::connect_non_secure(host, port, config).await
        }

        #[cfg(not(feature = "tls"))]
        Self::connect_non_secure(host, port, config).await
    }

    pub async fn connect_non_secure(host: &str, port: u16, config: &Config) -> Result<Self> {
        let (reader, writer) = tcp_connect(host, port, config).await?;
        let framed_read = FramedRead::new(reader, ValueDecoder);
        let framed_write = FramedWrite::new(writer, CommandEncoder);
        Ok(Streams::Tcp(framed_read, framed_write))
    }
}

pub struct StandaloneConnection {
    host: String,
    port: u16,
    config: Config,
    streams: Streams,
    buffer: BytesMut,
}

impl StandaloneConnection {
    pub async fn connect(host: &str, port: u16, config: &Config) -> Result<Self> {
        let streams = Streams::connect(host, port, config).await?;

        let mut connection = Self {
            host: host.to_owned(),
            port,
            config: config.clone(),
            streams,
            buffer: BytesMut::new(),
        };

        connection.post_connect().await?;

        Ok(connection)
    }

    pub async fn write(&mut self, command: &Command) -> Result<()> {
        if log_enabled!(Level::Debug) {
            if self.config.connection_name.is_empty() {
                debug!("[{}:{}] Sending {command:?}", self.host, self.port);
            } else {
                debug!("[{}] Sending {command:?}", self.config.connection_name);
            }
        }
        match &mut self.streams {
            Streams::Tcp(_, framed_write) => framed_write.send(command).await,
            #[cfg(feature = "tls")]
            Streams::TcpTls(_, framed_write) => framed_write.send(command).await,
        }
    }

    pub async fn write_batch(
        &mut self,
        commands: impl Iterator<Item = &mut Command>,
        _retry_reasons: &[RetryReason],
    ) -> Result<()> {
        self.buffer.clear();

        let command_encoder = match &mut self.streams {
            Streams::Tcp(_, framed_write) => framed_write.encoder_mut(),
            #[cfg(feature = "tls")]
            Streams::TcpTls(_, framed_write) => framed_write.encoder_mut(),
        };

        #[cfg(debug_assertions)]
        let mut kill_connection = false;

        for command in commands {
            if log_enabled!(Level::Debug) {
                if self.config.connection_name.is_empty() {
                    debug!("[{}:{}] Sending {command:?}", self.host, self.port);
                } else {
                    debug!("[{}] Sending {command:?}", self.config.connection_name);
                }
            }

            #[cfg(debug_assertions)]
            if command.kill_connection_on_write > 0 {
                kill_connection = true;
                command.kill_connection_on_write -= 1;
            }

            command_encoder.encode(command, &mut self.buffer)?;
        }

        #[cfg(debug_assertions)]
        if kill_connection {
            let client_id = self.client_id().await?;
            let mut config = self.config.clone();
            config.connection_name = "killer".to_owned();
            let mut connection = StandaloneConnection::connect(&self.host, self.port, &config).await?;
            connection.client_kill(crate::commands::ClientKillOptions::default().id(client_id)).await?;
        }

        match &mut self.streams {
            Streams::Tcp(_, framed_write) => framed_write.get_mut().write_all(&self.buffer).await?,
            #[cfg(feature = "tls")]
            Streams::TcpTls(_, framed_write) => {
                framed_write.get_mut().write_all(&self.buffer).await?
            }
        }

        Ok(())
    }

    pub async fn read(&mut self) -> Option<Result<Value>> {
        if let Some(value) = match &mut self.streams {
            Streams::Tcp(framed_read, _) => framed_read.next().await,
            #[cfg(feature = "tls")]
            Streams::TcpTls(framed_read, _) => framed_read.next().await,
        } {
            if log_enabled!(Level::Debug) {
                let connection_name = if self.config.connection_name.is_empty() {
                    format!("{}:{}", self.host, self.port)
                } else {
                    self.config.connection_name.clone()
                };
                match &value {
                    Ok(Value::Array(array)) => {
                        if array.len() > 100 {
                            debug!(
                                "[{connection_name}] Received result Array(Vec([...]))",
                            );
                        } else {
                            debug!("[{connection_name}] Received result {value:?}");
                        }
                    }
                    _ => debug!("[{connection_name}] Received result {value:?}"),
                }
            }
            Some(value)
        } else {
            None
        }
    }

    pub async fn reconnect(&mut self) -> Result<()> {
        self.streams = Streams::connect(&self.host, self.port, &self.config).await?;
        self.post_connect().await?;

        Ok(())

        // TODO improve reconnection strategy with multiple retries
    }

    async fn post_connect(&mut self) -> Result<()> {
        // RESP3
        let mut hello_options = HelloOptions::new(3);

        // authentication
        if let Some(ref password) = self.config.password {
            hello_options = hello_options.auth(match &self.config.username {
                Some(username) => username.clone(),
                None => "default".to_owned(),
            }, password.clone());
        }

        // connection name
        if !self.config.connection_name.is_empty() {
            hello_options = hello_options.set_name(self.config.connection_name.clone());
        }

        self.hello(hello_options).await?;

        // select database
        if self.config.database != 0 {
            self.select(self.config.database).await?;
        }

        Ok(())
    }
}

impl<'a, R> IntoFuture for PreparedCommand<'a, StandaloneConnection, R>
where
    R: FromValue + Send + 'a,
{
    type Output = Result<R>;
    type IntoFuture = Future<'a, R>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            self.executor.write(&self.command).await?;

            self.executor
                .read()
                .await
                .ok_or_else(|| Error::Client("Disconnected by peer".to_owned()))?
                .into_result()?
                .into()
        })
    }
}

impl ClusterCommands for StandaloneConnection {}
impl ConnectionCommands for StandaloneConnection {}
impl SentinelCommands for StandaloneConnection {}
impl ServerCommands for StandaloneConnection {}
