use crate::{
    resp::{Array, BulkString, Command, Value},
    ClusterCommands, ClusterConfig, ClusterShardResult, CommandInfoManager, CommandTip, Config,
    Error, RedisError, RedisErrorKind, RequestPolicy, ResponsePolicy, Result, RetryReason,
    StandaloneConnection,
};
use futures::{channel::mpsc, SinkExt, Stream, StreamExt};
use log::{debug, info, trace, warn};
use rand::Rng;
use smallvec::{smallvec, SmallVec};
use std::{
    cmp::Ordering,
    fmt::{Debug, Formatter},
    iter::zip,
    pin::Pin,
    task::{Context, Poll},
};

struct Node {
    pub id: String,
    pub address: (String, u16),
    pub connection: StandaloneConnection,
}

impl Debug for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("address", &self.address)
            .finish()
    }
}

#[derive(Debug)]
struct Shard {
    /// Master & replica nodes. Master is always the first node
    pub nodes: Vec<Node>,
}

#[derive(Debug)]
struct SlotRange {
    pub slot_range: (u16, u16),
    pub shard_index: usize,
}

#[derive(Debug)]
struct RequestInfo {
    pub command_name: String,
    pub keys: SmallVec<[String; 10]>,
    pub sub_requests: SmallVec<[SubRequest; 10]>,
}

#[derive(Debug)]
struct SubRequest {
    pub shard_index: usize,
    pub node_index: usize,
    pub keys: SmallVec<[String; 10]>,
}

type RequestSender = mpsc::UnboundedSender<RequestInfo>;
type RequestReceiver = mpsc::UnboundedReceiver<RequestInfo>;

struct RequestStream {
    receiver: RequestReceiver,
}

impl Stream for RequestStream {
    type Item = RequestInfo;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().receiver.poll_next_unpin(cx)
    }
}

/// Cluster connection
/// read & write_batch functions are implemented following Redis Command Tips
/// See <https://redis.io/docs/reference/command-tips/>
pub struct ClusterConnection {
    cluster_config: ClusterConfig,
    config: Config,
    shards: Vec<Shard>,
    slot_ranges: Vec<SlotRange>,
    command_info_manager: CommandInfoManager,
    request_sender: RequestSender,
    request_stream: RequestStream,
}

impl ClusterConnection {
    pub async fn connect(
        cluster_config: &ClusterConfig,
        config: &Config,
    ) -> Result<ClusterConnection> {
        let (mut shards, slot_ranges) = connect_to_cluster(cluster_config, config).await?;

        let command_info_manager =
            CommandInfoManager::initialize(&mut shards[0].nodes[0].connection).await?;

        let (request_sender, request_receiver): (RequestSender, RequestReceiver) =
            mpsc::unbounded();

        let request_stream = RequestStream {
            receiver: request_receiver,
        };

        debug!("Cluster connected: shards={shards:?}, slot_ranges={slot_ranges:?}");

        Ok(ClusterConnection {
            cluster_config: cluster_config.clone(),
            config: config.clone(),
            shards,
            slot_ranges,
            command_info_manager,
            request_sender,
            request_stream,
        })
    }

    pub async fn write_batch(
        &mut self,
        commands: impl Iterator<Item = &Command>,
        retry_reasons: &[RetryReason],
    ) -> Result<()> {
        if retry_reasons.iter().any(|r| {
            matches!(
                r,
                RetryReason::Moved {
                    hash_slot: _,
                    address: _
                }
            )
        }) {
            self.reconnect().await?;
        }

        let ask_reasons = retry_reasons.iter().filter_map(|r| {
            if let RetryReason::Ask { hash_slot, address } = r {
                Some((*hash_slot, address.clone()))
            } else {
                None
            }
        }).collect::<Vec<_>>();

        for command in commands {
            debug!("Analyzing command {command:?}");

            let command_info = self.command_info_manager.get_command_info(command);

            let command_info = if let Some(command_info) = command_info {
                command_info
            } else {
                return Err(Error::Client(format!("Unknown command {}", command.name)));
            };

            let command_name = command_info.name.to_string();

            let request_policy = command_info.command_tips.iter().find_map(|tip| {
                if let CommandTip::RequestPolicy(request_policy) = tip {
                    Some(request_policy)
                } else {
                    None
                }
            });

            let keys = self
                .command_info_manager
                .extract_keys(command, &mut self.shards[0].nodes[0].connection)
                .await?;
            let slots = Self::hash_slots(&keys);

            debug!("keys: {keys:?}, slots: {slots:?}");

            if let Some(request_policy) = request_policy {
                match request_policy {
                    RequestPolicy::AllNodes => {
                        self.request_policy_all_nodes(command, &command_name, keys)
                            .await?;
                    }
                    RequestPolicy::AllShards => {
                        self.request_policy_all_shards(command, &command_name, keys)
                            .await?;
                    }
                    RequestPolicy::MultiShard => {
                        self.request_policy_multi_shard(command, &command_name, keys, slots, &ask_reasons)
                            .await?;
                    }
                    RequestPolicy::Special => {
                        self.request_policy_special(command, command_name, keys, slots);
                    }
                }
            } else {
                self.no_request_policy(command, command_name, keys, slots, &ask_reasons)
                    .await?;
            }
        }

        Ok(())
    }

    /// The client should execute the command on all master shards (e.g., the DBSIZE command).
    /// This tip is in-use by commands that don't accept key name arguments.
    /// The command operates atomically per shard.
    async fn request_policy_all_shards(
        &mut self,
        command: &Command,
        command_name: &str,
        keys: SmallVec<[String; 10]>,
    ) -> Result<()> {
        for shard in &mut self.shards {
            shard.nodes[0].connection.write(command).await?;
        }
        let request_info = RequestInfo {
            command_name: command_name.to_string(),
            sub_requests: (0..self.shards.len())
                .map(|i| SubRequest {
                    shard_index: i,
                    node_index: 0,
                    keys: smallvec![],
                })
                .collect(),
            keys,
        };
        self.request_sender.send(request_info).await?;
        Ok(())
    }

    /// The client should execute the command on all nodes - masters and replicas alike.
    /// An example is the CONFIG SET command.
    /// This tip is in-use by commands that don't accept key name arguments.
    /// The command operates atomically per shard.
    async fn request_policy_all_nodes(
        &mut self,
        command: &Command,
        command_name: &str,
        keys: SmallVec<[String; 10]>,
    ) -> Result<()> {
        if self.shards[0].nodes.len() == 1 {
            self.connect_replicas().await?;
        }
        let mut sub_requests = SmallVec::<[SubRequest; 10]>::new();
        for (shard_index, shard) in &mut self.shards.iter_mut().enumerate() {
            for (node_index, node) in &mut shard.nodes.iter_mut().enumerate() {
                node.connection.write(command).await?;
                sub_requests.push(SubRequest {
                    shard_index,
                    node_index,
                    keys: smallvec![],
                });
            }
        }
        let request_info = RequestInfo {
            command_name: command_name.to_string(),
            sub_requests,
            keys,
        };
        self.request_sender.send(request_info).await?;
        Ok(())
    }

    /// The client should execute the command on several shards.
    /// The shards that execute the command are determined by the hash slots of its input key name arguments.
    /// Examples for such commands include MSET, MGET and DEL.
    /// However, note that SUNIONSTORE isn't considered as multi_shard because all of its keys must belong to the same hash slot.
    async fn request_policy_multi_shard(
        &mut self,
        command: &Command,
        command_name: &str,
        keys: SmallVec<[String; 10]>,
        slots: SmallVec<[u16; 10]>,
        ask_reasons: &[(u16, (String, u16))],
    ) -> Result<()> {
        let mut shard_slot_keys_ask = (0..keys.len())
            .map(|i| {
                let (shard_index, should_ask) = self.get_shard_index_by_slot(slots[i], ask_reasons);
                (
                    shard_index,
                    slots[i],
                    keys[i].clone(),
                    should_ask,
                )
            })
            .collect::<Vec<_>>();

        shard_slot_keys_ask.sort();
        trace!("shard_slot_keys_ask: {shard_slot_keys_ask:?}");

        let mut last_slot = u16::MAX;
        let mut current_slot_keys = SmallVec::<[String; 10]>::new();
        let mut sub_requests = SmallVec::<[SubRequest; 10]>::new();
        let mut last_shard_index = 0;
        let mut last_should_ask = false;

        let mut connection: &mut StandaloneConnection =
            &mut self.shards[last_shard_index].nodes[0].connection;

        for (shard_index, slot, key, should_ask) in &shard_slot_keys_ask {
            if *slot != last_slot {
                if !current_slot_keys.is_empty() {
                    if last_should_ask {
                        connection.asking().await?;
                    }

                    let shard_command = self
                        .command_info_manager
                        .prepare_command_for_shard(command, current_slot_keys.iter())?;
                    connection.write(&shard_command).await?;
                    sub_requests.push(SubRequest {
                        shard_index: last_shard_index,
                        node_index: 0,
                        keys: current_slot_keys.clone(),
                    });

                    current_slot_keys.clear();
                }

                last_slot = *slot;
                last_should_ask = *should_ask;
            }

            current_slot_keys.push(key.clone());

            if *shard_index != last_shard_index {
                connection = &mut self.shards[*shard_index].nodes[0].connection;
                last_shard_index = *shard_index;
            }
        }

        if last_should_ask {
            connection.asking().await?;
        }

        let shard_command = self
            .command_info_manager
            .prepare_command_for_shard(command, current_slot_keys.iter())?;

        connection.write(&shard_command).await?;

        sub_requests.push(SubRequest {
            shard_index: last_shard_index,
            node_index: 0,
            keys: current_slot_keys.clone(),
        });

        let request_info = RequestInfo {
            command_name: command_name.to_string(),
            keys,
            sub_requests,
        };

        trace!("{request_info:?}");

        self.request_sender.send(request_info).await?;

        Ok(())
    }

    async fn no_request_policy(
        &mut self,
        command: &Command,
        command_name: String,
        keys: SmallVec<[String; 10]>,
        slots: SmallVec<[u16; 10]>,
        ask_reasons: &[(u16, (String, u16))]
    ) -> Result<()> {
        // test if all slots are equal
        if slots.windows(2).all(|s| s[0] == s[1]) {
            let (shard_index, should_ask) = if slots.is_empty() {
                (rand::thread_rng().gen_range(0..self.shards.len()), false)
            } else {
                self.get_shard_index_by_slot(slots[0], ask_reasons)
            };

            let connection = &mut self.shards[shard_index].nodes[0].connection;

            if should_ask {
                connection.asking().await?;
            }
            connection.write(command).await?;

            let request_info = RequestInfo {
                command_name: command_name.to_string(),
                sub_requests: smallvec![SubRequest {
                    shard_index,
                    node_index: 0,
                    keys: keys.clone()
                }],
                keys,
            };
            self.request_sender.send(request_info).await?;
        } else {
            return Err(Error::Client(format!(
                "Cannot send command {} with mistmatched key slots",
                command_name
            )));
        }

        Ok(())
    }

    fn request_policy_special(
        &mut self,
        _command: &Command,
        _command_name: String,
        _keys: SmallVec<[String; 10]>,
        _slots: SmallVec<[u16; 10]>,
    ) {
        todo!("Command not yet supported in cluster mode")
    }

    pub fn read(&mut self) -> futures::future::BoxFuture<'_, Option<Result<Value>>> {
        Box::pin(async move {
            let request_info = self.request_stream.next().await?;

            let mut sub_results =
                Vec::<Result<Value>>::with_capacity(request_info.sub_requests.len());
            let mut is_none = false;
            let mut retry_reasons = SmallVec::<[RetryReason; 1]>::new();

            // make sure to read all response for each sub_request with no early return
            for sub_request in &request_info.sub_requests {
                let shard = &mut self.shards[sub_request.shard_index];
                let result = shard.nodes[sub_request.node_index].connection.read().await;

                if let Some(result) = result {
                    match &result {
                        Ok(Value::Error(RedisError {
                            kind: RedisErrorKind::Ask { hash_slot, address },
                            description: _,
                        })) => retry_reasons.push(RetryReason::Ask {
                            hash_slot: *hash_slot,
                            address: address.clone(),
                        }),
                        Ok(Value::Error(RedisError {
                            kind: RedisErrorKind::Moved { hash_slot, address },
                            description: _,
                        })) => retry_reasons.push(RetryReason::Moved {
                            hash_slot: *hash_slot,
                            address: address.clone(),
                        }),
                        _ => sub_results.push(result),
                    }
                } else {
                    is_none = true;
                }
            }

            // from here we can early return
            if is_none {
                return None;
            }

            if !retry_reasons.is_empty() {
                debug!(
                    "read failed and will be retried. reasons: {:?}",
                    retry_reasons
                );
                return Some(Err(Error::Retry(retry_reasons)));
            }

            let command_name = &request_info.command_name;
            let command_info = self
                .command_info_manager
                .get_command_info_by_name(command_name);

            let command_info = if let Some(command_info) = command_info {
                command_info
            } else {
                return Some(Err(Error::Client(format!(
                    "Unknown command {}",
                    command_name
                ))));
            };

            let response_policy = command_info.command_tips.iter().find_map(|tip| {
                if let CommandTip::ResponsePolicy(response_policy) = tip {
                    Some(response_policy)
                } else {
                    None
                }
            });

            // The response_policy tip is set for commands that reply with scalar data types,
            // or when it's expected that clients implement a non-default aggregate.
            if let Some(response_policy) = response_policy {
                match response_policy {
                    ResponsePolicy::OneSucceeded => {
                        self.response_policy_one_succeeded(sub_results).await
                    }
                    ResponsePolicy::AllSucceeded => {
                        self.response_policy_all_succeeded(sub_results).await
                    }
                    ResponsePolicy::AggLogicalAnd => {
                        self.response_policy_agg(
                            sub_results,
                            |a, b| i64::from(a == 1 && b == 1),
                        )
                        .await
                    }
                    ResponsePolicy::AggLogicalOr => {
                        self.response_policy_agg(
                            sub_results,
                            |a, b| if a == 0 && b == 0 { 0 } else { 1 },
                        )
                        .await
                    }
                    ResponsePolicy::AggMin => self.response_policy_agg(sub_results, i64::min).await,
                    ResponsePolicy::AggMax => self.response_policy_agg(sub_results, i64::max).await,
                    ResponsePolicy::AggSum => {
                        self.response_policy_agg(sub_results, |a, b| a + b).await
                    }
                    ResponsePolicy::Special => self.response_policy_special(sub_results).await,
                }
            } else {
                self.no_response_policy(sub_results, &request_info).await
            }
        })
    }

    async fn response_policy_one_succeeded(
        &mut self,
        sub_results: Vec<Result<Value>>,
    ) -> Option<Result<Value>> {
        let mut result: Result<Value> = Ok(Value::BulkString(BulkString::Nil));

        for sub_result in sub_results {
            if let Err(_) | Ok(Value::Error(_)) = sub_result {
                result = sub_result;
            } else {
                return Some(sub_result);
            }
        }

        Some(result)
    }

    async fn response_policy_all_succeeded(
        &mut self,
        sub_results: Vec<Result<Value>>,
    ) -> Option<Result<Value>> {
        let mut result: Result<Value> = Ok(Value::BulkString(BulkString::Nil));

        for sub_result in sub_results {
            if let Err(_) | Ok(Value::Error(_)) = sub_result {
                return Some(sub_result);
            } else {
                result = sub_result;
            }
        }

        Some(result)
    }

    async fn response_policy_agg<F>(
        &mut self,
        sub_results: Vec<Result<Value>>,
        f: F,
    ) -> Option<Result<Value>>
    where
        F: Fn(i64, i64) -> i64,
    {
        let mut result = Value::BulkString(BulkString::Nil);

        for sub_result in sub_results {
            result = match sub_result {
                Ok(Value::Error(_)) => {
                    return Some(sub_result);
                }
                Ok(value) => match (value, result) {
                    (Value::Integer(v), Value::Integer(r)) => Value::Integer(f(v, r)),
                    (Value::Integer(v), Value::BulkString(BulkString::Nil)) => Value::Integer(v),
                    (Value::Array(Array::Vec(v)), Value::Array(Array::Vec(mut r)))
                        if v.len() == r.len() =>
                    {
                        for i in 0..v.len() {
                            match (&v[i], &r[i]) {
                                (Value::Integer(vi), Value::Integer(ri)) => {
                                    r[i] = Value::Integer(f(*vi, *ri));
                                }
                                _ => {
                                    return Some(Err(Error::Client("Unexpected value".to_owned())));
                                }
                            }
                        }
                        Value::Array(Array::Vec(r))
                    }
                    (Value::Array(Array::Vec(v)), Value::BulkString(BulkString::Nil)) => {
                        Value::Array(Array::Vec(v))
                    }
                    _ => {
                        return Some(Err(Error::Client("Unexpected value".to_owned())));
                    }
                },
                Err(_) => {
                    return Some(sub_result);
                }
            };
        }

        Some(Ok(result))
    }

    async fn response_policy_special(
        &mut self,
        _sub_results: Vec<Result<Value>>,
    ) -> Option<Result<Value>> {
        todo!("Command not yet supported in cluster mode");
    }

    async fn no_response_policy(
        &mut self,
        sub_results: Vec<Result<Value>>,
        request_info: &RequestInfo,
    ) -> Option<Result<Value>> {
        if sub_results.len() == 1 {
            // when there is a single sub request, we just read the response
            // on the right connection. For example, GET's reply
            Some(sub_results.into_iter().next()?)
        } else if request_info.keys.is_empty() {
            // The command doesn't accept key name arguments:
            // the client can aggregate all replies within a single nested data structure.
            // For example, the array replies we get from calling KEYS against all shards.
            // These should be packed in a single in no particular order.
            let mut values = Vec::<Value>::new();
            for sub_result in sub_results {
                match sub_result {
                    Ok(Value::Array(Array::Vec(v))) => {
                        values.extend(v);
                    }
                    Err(_) | Ok(Value::Error(_)) => {
                        return Some(sub_result);
                    }
                    _ => {
                        return Some(Err(Error::Client(format!(
                            "Unexpected result {sub_result:?}"
                        ))));
                    }
                }
            }

            Some(Ok(Value::Array(Array::Vec(values))))
        } else {
            // For commands that accept one or more key name arguments:
            // the client needs to retain the same order of replies as the input key names.
            // For example, MGET's aggregated reply.
            let mut results = SmallVec::<[(&String, Value); 10]>::new();

            for (sub_result, sub_request) in zip(sub_results, &request_info.sub_requests) {
                match sub_result {
                    Ok(Value::Array(Array::Vec(values)))
                        if sub_request.keys.len() == values.len() =>
                    {
                        results.extend(zip(&sub_request.keys, values))
                    }
                    Err(_) | Ok(Value::Error(_)) => return Some(sub_result),
                    _ => {
                        return Some(Err(Error::Client(format!(
                            "Unexpected result {:?}",
                            sub_result
                        ))))
                    }
                }
            }

            results.sort_by(|(k1, _), (k2, _)| {
                request_info
                    .keys
                    .iter()
                    .position(|k| k == *k1)
                    .cmp(&request_info.keys.iter().position(|k| k == *k2))
            });

            let values = results.into_iter().map(|(_, v)| v).collect::<Vec<_>>();
            Some(Ok(Value::Array(Array::Vec(values))))
        }
    }

    pub async fn reconnect(&mut self) -> Result<()> {
        info!("Reconnecting to cluster...");
        let (shards, slot_ranges) = connect_to_cluster(&self.cluster_config, &self.config).await?;
        info!("Reconnected to cluster!");

        self.shards = shards;
        self.slot_ranges = slot_ranges;

        Ok(())

        // TODO improve reconnection strategy with multiple retries
    }

    async fn connect_replicas(&mut self) -> Result<()> {
        let shard_infos: Vec<ClusterShardResult> =
            self.shards[0].nodes[0].connection.cluster_shards().await?;

        for shard_info in shard_infos {
            let shard = if let Some(shard) = self
                .shards
                .iter_mut()
                .find(|s| s.nodes[0].id == shard_info.nodes[0].id)
            {
                shard
            } else {
                return Err(Error::Client(format!(
                    "Cannot find shard into for slot range {:?}",
                    shard_info.slots
                )));
            };

            for node in shard_info.nodes.into_iter().filter(|n| n.role == "replica") {
                let port = match (node.port, node.tls_port) {
                    (None, Some(port)) => port,
                    (Some(port), None) => port,
                    _ => {
                        return Err(Error::Client("Cluster misconfiguration".to_owned()));
                    }
                };

                let connection =
                    StandaloneConnection::connect(&node.ip, port, &self.config).await?;

                shard.nodes.push(Node {
                    id: node.id,
                    address: (node.ip.clone(), port),
                    connection,
                });
            }
        }

        Ok(())
    }

    fn get_shard_index_by_slot(&mut self, slot: u16, ask_reasons: &[(u16, (String, u16))]) -> (usize, bool) {
        let ask_reason = ask_reasons.iter().find(|(hash_slot, (_ip, _port))| *hash_slot == slot);

        if let Some((_hash_slot, address)) = ask_reason {
            let shard_index = self.shards.iter().position(|s| s.nodes.iter().any(|n| n.address == *address)).unwrap();
            (shard_index, true)
        } else {
            let shard_index = self.slot_ranges[self
                .slot_ranges
                .binary_search_by(|s| {
                    if s.slot_range.0 > slot {
                        Ordering::Greater
                    } else if s.slot_range.1 < slot {
                        Ordering::Less
                    } else {
                        Ordering::Equal
                    }
                })
                .unwrap()]
            .shard_index;
            (shard_index, false)
        }
    }

    fn hash_slots(keys: &[String]) -> SmallVec<[u16; 10]> {
        keys.iter().map(|k| Self::hash_slot(k)).collect()
    }

    /// Implement hash_slot algorithm
    /// see. https://redis.io/docs/reference/cluster-spec/#hash-tags
    fn hash_slot(key: &str) -> u16 {
        let mut key = key;

        // { found
        if let Some(s) = key.find('{') {
            // } found
            if let Some(e) = key[s + 1..].find('}') {
                // hash tag non empty
                if e != 0 {
                    key = &key[s + 1..s + 1 + e];
                }
            }
        }

        Self::crc16(key) % 16384
    }

    fn crc16(str: &str) -> u16 {
        crc16::State::<crc16::XMODEM>::calculate(str.as_bytes())
    }
}

async fn connect_to_cluster(
    cluster_config: &ClusterConfig,
    config: &Config,
) -> Result<(Vec<Shard>, Vec<SlotRange>)> {
    debug!("Discovering cluster shard and slots...");

    let mut shard_info_list: Option<Vec<ClusterShardResult>> = None;

    for node_config in &cluster_config.nodes {
        match StandaloneConnection::connect(&node_config.0, node_config.1, config).await {
            Ok(mut connection) => match connection.cluster_shards().await {
                Ok(si) => {
                    shard_info_list = Some(si);
                    break;
                }
                Err(e) => warn!(
                    "Cannot execute `cluster_shards` on node ({}:{}): {}",
                    node_config.0, node_config.1, e
                ),
            },
            Err(e) => warn!(
                "Cannot connect to node ({}:{}): {}",
                node_config.0, node_config.1, e
            ),
        }
    }

    let shard_info_list = if let Some(shard_info_list) = shard_info_list {
        shard_info_list
    } else {
        return Err(Error::Client("Cluster misconfiguration".to_owned()));
    };

    let mut shards = Vec::<Shard>::new();
    let mut slot_ranges = Vec::<SlotRange>::new();

    for (shard_index, shard_info) in shard_info_list.into_iter().enumerate() {
        let master_info = if let Some(master_info) = shard_info.nodes.into_iter().next() {
            master_info
        } else {
            return Err(Error::Client("Cluster misconfiguration".to_owned()));
        };

        let port = match (master_info.port, master_info.tls_port) {
            (None, Some(port)) => port,
            (Some(port), None) => port,
            _ => {
                return Err(Error::Client("Cluster misconfiguration".to_owned()));
            }
        };

        let connection = StandaloneConnection::connect(&master_info.ip, port, config).await?;

        shards.push(Shard {
            nodes: vec![Node {
                id: master_info.id,
                address: (master_info.ip, port),
                connection,
            }],
        });

        slot_ranges.extend(shard_info.slots.iter().map(|s| SlotRange {
            slot_range: *s,
            shard_index,
        }));
    }

    slot_ranges.sort_by_key(|s| s.slot_range.0);

    Ok((shards, slot_ranges))
}
