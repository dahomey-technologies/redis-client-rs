use std::collections::HashMap;

use crate::{
    resp::{
        cmd, BulkString, CommandArgs, FromKeyValueValueArray, FromSingleValueArray, FromValue,
        HashMapExt, IntoArgs, KeyValueArgOrCollection, SingleArgOrCollection, Value,
    },
    CommandResult, Error, PrepareCommand, Result,
};

/// A group of Redis commands related to Server Management
/// # See Also
/// [Redis Server Management Commands](https://redis.io/commands/?group=server)
/// [ACL guide](https://redis.io/docs/manual/security/acl/)
pub trait ServerCommands<T>: PrepareCommand<T> {
    /// The command shows the available ACL categories if called without arguments.
    /// If a category name is given, the command shows all the Redis commands in the specified category.
    ///
    /// # Return
    /// A collection of ACL categories or a collection of commands inside a given category.
    ///
    /// # Errors
    /// The command may return an error if an invalid category name is given as argument.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-cat/>](https://redis.io/commands/acl-cat/)
    fn acl_cat<C, CC>(&self, options: AclCatOptions) -> CommandResult<T, CC>
    where
        C: FromValue,
        CC: FromSingleValueArray<C>,
    {
        self.prepare_command(cmd("ACL").arg("CAT").arg(options))
    }

    /// Delete all the specified ACL users and terminate all
    /// the connections that are authenticated with such users.
    ///
    /// # Return
    /// The number of users that were deleted.
    /// This number will not always match the number of arguments since certain users may not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-deluser/>](https://redis.io/commands/acl-deluser/)
    fn acl_deluser<U, UU>(&self, usernames: UU) -> CommandResult<T, usize>
    where
        U: Into<BulkString>,
        UU: SingleArgOrCollection<U>,
    {
        self.prepare_command(cmd("ACL").arg("DELUSER").arg(usernames))
    }

    /// Simulate the execution of a given command by a given user.
    ///
    /// # Return
    /// OK on success.
    /// An error describing why the user can't execute the command.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-dryrun/>](https://redis.io/commands/acl-dryrun/)
    fn acl_dryrun<U, C, R>(
        &self,
        username: U,
        command: C,
        options: AclDryRunOptions,
    ) -> CommandResult<T, R>
    where
        U: Into<BulkString>,
        C: Into<BulkString>,
        R: FromValue,
    {
        self.prepare_command(
            cmd("ACL")
                .arg("DRYRUN")
                .arg(username)
                .arg(command)
                .arg(options),
        )
    }

    /// Generates a password starting from /dev/urandom if available,
    /// otherwise (in systems without /dev/urandom) it uses a weaker
    /// system that is likely still better than picking a weak password by hand.
    ///
    /// # Return
    /// by default 64 bytes string representing 256 bits of pseudorandom data.
    /// Otherwise if an argument if needed, the output string length is the number
    /// of specified bits (rounded to the next multiple of 4) divided by 4.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-genpass/>](https://redis.io/commands/acl-genpass/)
    fn acl_genpass<R: FromValue>(&self, options: AclGenPassOptions) -> CommandResult<T, R> {
        self.prepare_command(cmd("ACL").arg("GENPASS").arg(options))
    }

    /// The command returns all the rules defined for an existing ACL user.
    ///
    /// # Return
    /// A collection of ACL rule definitions for the user.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-getuser/>](https://redis.io/commands/acl-getuser/)
    fn acl_getuser<U, RR>(&self, username: U) -> CommandResult<T, RR>
    where
        U: Into<BulkString>,
        RR: FromKeyValueValueArray<String, Value>,
    {
        self.prepare_command(cmd("ACL").arg("GETUSER").arg(username))
    }

    /// The command shows the currently active ACL rules in the Redis server.
    ///
    /// # Return
    /// An array of strings.
    /// Each line in the returned array defines a different user, and the
    /// format is the same used in the redis.conf file or the external ACL file
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-list/>](https://redis.io/commands/acl-list/)
    fn acl_list(&self) -> CommandResult<T, Vec<String>> {
        self.prepare_command(cmd("ACL").arg("LIST"))
    }

    /// When Redis is configured to use an ACL file (with the aclfile configuration option),
    /// this command will reload the ACLs from the file, replacing all the current ACL rules
    /// with the ones defined in the file.
    ///
    /// # Return
    /// An array of strings.
    /// Each line in the returned array defines a different user, and the
    /// format is the same used in the redis.conf file or the external ACL file
    ///
    /// # Errors
    /// The command may fail with an error for several reasons:
    /// - if the file is not readable,
    /// - if there is an error inside the file, and in such case the error will be reported to the user in the error.
    /// - Finally the command will fail if the server is not configured to use an external ACL file.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-load/>](https://redis.io/commands/acl-load/)
    fn acl_load(&self) -> CommandResult<T, ()> {
        self.prepare_command(cmd("ACL").arg("LOAD"))
    }

    /// The command shows a list of recent ACL security events
    ///
    /// # Return
    /// A key/value collection of ACL security events.
    /// Empty collection when called with the [`reset`](crate::AclLogOptions::reset) option
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-log/>](https://redis.io/commands/acl-log/)
    fn acl_log<EE>(&self, options: AclLogOptions) -> CommandResult<T, Vec<EE>>
    where
        EE: FromKeyValueValueArray<String, Value>,
    {
        self.prepare_command(cmd("ACL").arg("LOG").arg(options))
    }

    /// When Redis is configured to use an ACL file (with the aclfile configuration option),
    /// this command will save the currently defined ACLs from the server memory to the ACL file.
    ///
    /// # Errors
    /// The command may fail with an error for several reasons:
    /// - if the file cannot be written
    /// - if the server is not configured to use an external ACL file.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-save/>](https://redis.io/commands/acl-save/)
    fn acl_save(&self) -> CommandResult<T, ()> {
        self.prepare_command(cmd("ACL").arg("SAVE"))
    }

    /// Create an ACL user with the specified rules or modify the rules of an existing user.
    ///
    /// # Errors
    /// If the rules contain errors, the error is returned.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-setuser/>](https://redis.io/commands/acl-setuser/)
    fn acl_setuser<U, R, RR>(&self, username: U, rules: RR) -> CommandResult<T, ()>
    where
        U: Into<BulkString>,
        R: Into<BulkString>,
        RR: SingleArgOrCollection<R>,
    {
        self.prepare_command(cmd("ACL").arg("SETUSER").arg(username).arg(rules))
    }

    /// The command shows a list of all the usernames of the currently configured users in the Redis ACL system.
    ///
    /// # Return
    /// A collection of usernames
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-users/>](https://redis.io/commands/acl-users/)
    fn acl_users<U, UU>(&self) -> CommandResult<T, UU>
    where
        U: FromValue,
        UU: FromSingleValueArray<U>,
    {
        self.prepare_command(cmd("ACL").arg("USERS"))
    }

    /// Return the username the current connection is authenticated with.
    ///
    /// # Return
    /// The username of the current connection.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-whoami/>](https://redis.io/commands/acl-whoami/)
    fn acl_whoami<U: FromValue>(&self) -> CommandResult<T, U> {
        self.prepare_command(cmd("ACL").arg("WHOAMI"))
    }

    /// Return an array with details about every Redis command.
    ///
    /// # Return
    /// A nested list of command details.
    /// The order of commands in the array is random.
    ///
    /// # See Also
    /// [<https://redis.io/commands/command/>](https://redis.io/commands/command/)
    fn command(&self) -> CommandResult<T, Vec<CommandInfo>> {
        self.prepare_command(cmd("COMMAND"))
    }

    /// Number of total commands in this Redis server.
    ///
    /// # Return
    /// number of commands returned by [`command`](crate::ServerCommands::command)
    ///
    /// # See Also
    /// [<https://redis.io/commands/command-count/>](https://redis.io/commands/command-count/)
    fn command_count(&self) -> CommandResult<T, usize> {
        self.prepare_command(cmd("COMMAND").arg("COUNT"))
    }

    /// Number of total commands in this Redis server.
    ///
    /// # Return
    /// map key=command name, value=command doc
    ///
    /// # See Also
    /// [<https://redis.io/commands/command-docs/>](https://redis.io/commands/command-docs/)
    fn command_docs<N, NN, DD>(&self, command_names: NN) -> CommandResult<T, DD>
    where
        N: Into<BulkString>,
        NN: SingleArgOrCollection<N>,
        DD: FromKeyValueValueArray<String, CommandDoc>,
    {
        self.prepare_command(cmd("COMMAND").arg("DOCS").arg(command_names))
    }

    /// A helper command to let you find the keys from a full Redis command.
    ///
    /// # Return
    /// list of keys from your command.
    ///
    /// # See Also
    /// [<https://redis.io/commands/command-_getkeys/>](https://redis.io/commands/command-_getkeys/)
    fn command_getkeys<A, AA, KK>(&self, args: AA) -> CommandResult<T, KK>
    where
        A: Into<BulkString>,
        AA: SingleArgOrCollection<A>,
        KK: FromSingleValueArray<String>,
    {
        self.prepare_command(cmd("COMMAND").arg("GETKEYS").arg(args))
    }

    /// A helper command to let you find the keys from a full Redis command together with flags indicating what each key is used for.
    ///
    /// # Return
    /// map of keys with their flags from your command.
    ///
    /// # See Also
    /// [<https://redis.io/commands/command-getkeysandflags/>](https://redis.io/commands/command-getkeysandflags/)
    fn command_getkeysandflags<A, AA, KK>(&self, args: AA) -> CommandResult<T, KK>
    where
        A: Into<BulkString>,
        AA: SingleArgOrCollection<A>,
        KK: FromKeyValueValueArray<String, Vec<String>>,
    {
        self.prepare_command(cmd("COMMAND").arg("GETKEYSANDFLAGS").arg(args))
    }

    /// Return an array with details about multiple Redis command.
    ///
    /// # Return
    /// A nested list of command details.
    ///
    /// # See Also
    /// [<https://redis.io/commands/command-info/>](https://redis.io/commands/command-info/)
    fn command_info<N, NN>(&self, command_names: NN) -> CommandResult<T, Vec<CommandInfo>>
    where
        N: Into<BulkString>,
        NN: SingleArgOrCollection<N>,
    {
        self.prepare_command(cmd("COMMAND").arg("INFO").arg(command_names))
    }

    /// Return an array of the server's command names based on optional filters
    ///
    /// # Return
    /// an array of the server's command names.
    ///
    /// # See Also
    /// [<https://redis.io/commands/command-list/>](https://redis.io/commands/command-list/)
    fn command_list<CC>(&self, options: CommandListOptions) -> CommandResult<T, CC>
    where
        CC: FromSingleValueArray<String>,
    {
        self.prepare_command(cmd("COMMAND").arg("LIST").arg(options))
    }

    /// Used to read the configuration parameters of a running Redis server.
    ///
    /// For every key that does not hold a string value or does not exist,
    /// the special value nil is returned. Because of this, the operation never fails.
    ///
    /// # Return
    /// Array reply: collection of the requested params with their matching values.
    ///
    /// # See Also
    /// [<https://redis.io/commands/config-get/>](https://redis.io/commands/config-get/)
    #[must_use]
    fn config_get<P, PP, V, VV>(&self, params: PP) -> CommandResult<T, VV>
    where
        P: Into<BulkString>,
        PP: SingleArgOrCollection<P>,
        V: FromValue,
        VV: FromKeyValueValueArray<String, V>,
    {
        self.prepare_command(cmd("CONFIG").arg("GET").arg(params))
    }

    /// Resets the statistics reported by Redis using the [`info`](crate::ServerCommands::info) command.
    ///
    /// # See Also
    /// [<https://redis.io/commands/config-resetstat/>](https://redis.io/commands/config-resetstat/)
    #[must_use]
    fn config_resetstat(&self) -> CommandResult<T, ()> {
        self.prepare_command(cmd("CONFIG").arg("RESETSTAT"))
    }

    /// Rewrites the redis.conf file the server was started with,
    /// applying the minimal changes needed to make it reflect the configuration currently used by the server,
    /// which may be different compared to the original one because of the use of the
    /// [`config_set`](crate::ServerCommands::config_set) command.
    ///
    /// # See Also
    /// [<https://redis.io/commands/config-rewrite/>](https://redis.io/commands/config-rewrite/)
    #[must_use]
    fn config_rewrite(&self) -> CommandResult<T, ()> {
        self.prepare_command(cmd("CONFIG").arg("REWRITE"))
    }

    /// Used in order to reconfigure the server at run time without the need to restart Redis.
    ///
    /// # See Also
    /// [<https://redis.io/commands/config-set/>](https://redis.io/commands/config-set/)
    #[must_use]
    fn config_set<P, V, C>(&self, configs: C) -> CommandResult<T, ()>
    where
        P: Into<BulkString>,
        V: Into<BulkString>,
        C: KeyValueArgOrCollection<P, V>,
    {
        self.prepare_command(cmd("CONFIG").arg("SET").arg(configs))
    }

    /// Return the number of keys in the currently-selected database.
    ///
    /// # See Also
    /// [<https://redis.io/commands/dbsize/>](https://redis.io/commands/dbsize/)
    #[must_use]
    fn dbsize(&self) -> CommandResult<T, usize> {
        self.prepare_command(cmd("DBSIZE"))
    }

    /// This command will start a coordinated failover between
    /// the currently-connected-to master and one of its replicas.
    ///
    /// # See Also
    /// [<https://redis.io/commands/failover/>](https://redis.io/commands/failover/)
    #[must_use]
    fn failover(&self, options: FailOverOptions) -> CommandResult<T, ()> {
        self.prepare_command(cmd("FAILOVER").arg(options))
    }

    /// Delete all the keys of the currently selected DB.
    ///
    /// # See Also
    /// [<https://redis.io/commands/flushdb/>](https://redis.io/commands/flushdb/)
    #[must_use]
    fn flushdb(&self, flushing_mode: FlushingMode) -> CommandResult<T, ()> {
        self.prepare_command(cmd("FLUSHDB").arg(flushing_mode))
    }

    /// Delete all the keys of all the existing databases, not just the currently selected one.
    ///
    /// # See Also
    /// [<https://redis.io/commands/flushall/>](https://redis.io/commands/flushall/)
    #[must_use]
    fn flushall(&self, flushing_mode: FlushingMode) -> CommandResult<T, ()> {
        self.prepare_command(cmd("FLUSHALL").arg(flushing_mode))
    }

    /// This command returns information and statistics about the server 
    /// in a format that is simple to parse by computers and easy to read by humans.
    ///
    /// # See Also
    /// [<https://redis.io/commands/info/>](https://redis.io/commands/info/)
    #[must_use]
    fn info<SS>(&self, sections: SS) -> CommandResult<T, String> 
    where
        SS: SingleArgOrCollection<InfoSection>
    {
        self.prepare_command(cmd("INFO").arg(sections))
    }

    /// Return the UNIX TIME of the last DB save executed with success.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lastsave/>](https://redis.io/commands/lastsave/)
    #[must_use]
    fn lastsave(&self) -> CommandResult<T, u64> 
    {
        self.prepare_command(cmd("LASTSAVE"))
    }

    /// The TIME command returns the current server time as a two items lists:
    /// a Unix timestamp and the amount of microseconds already elapsed in the current second.
    ///
    /// # See Also
    /// [<https://redis.io/commands/time/>](https://redis.io/commands/time/)
    #[must_use]
    fn time(&self) -> CommandResult<T, (u32, u32)> {
        self.prepare_command(cmd("TIME"))
    }
}

/// Database flushing mode
pub enum FlushingMode {
    Default,
    /// Flushes the database(s) asynchronously
    Async,
    /// Flushed the database(s) synchronously
    Sync,
}

impl Default for FlushingMode {
    fn default() -> Self {
        FlushingMode::Default
    }
}

impl IntoArgs for FlushingMode {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            FlushingMode::Default => args,
            FlushingMode::Async => args.arg("ASYNC"),
            FlushingMode::Sync => args.arg("SYNC"),
        }
    }
}

/// Options for the [`acl_cat`](crate::ServerCommands::acl_cat) command
#[derive(Default)]
pub struct AclCatOptions {
    command_args: CommandArgs,
}

impl AclCatOptions {
    #[must_use]
    pub fn category_name<C: Into<BulkString>>(self, category_name: C) -> Self {
        Self {
            command_args: self.command_args.arg(category_name),
        }
    }
}

impl IntoArgs for AclCatOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Options for the [`acl_dryrun`](crate::ServerCommands::acl_dryrun) command
#[derive(Default)]
pub struct AclDryRunOptions {
    command_args: CommandArgs,
}

impl AclDryRunOptions {
    #[must_use]
    pub fn arg<A, AA>(self, args: AA) -> Self
    where
        A: Into<BulkString>,
        AA: SingleArgOrCollection<A>,
    {
        Self {
            command_args: self.command_args.arg(args),
        }
    }
}

impl IntoArgs for AclDryRunOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Options for the [`acl_genpass`](crate::ServerCommands::acl_genpass) command
#[derive(Default)]
pub struct AclGenPassOptions {
    command_args: CommandArgs,
}

impl AclGenPassOptions {
    /// The command output is a hexadecimal representation of a binary string.
    /// By default it emits 256 bits (so 64 hex characters).
    /// The user can provide an argument in form of number of bits to emit from 1 to 1024 to change the output length.
    /// Note that the number of bits provided is always rounded to the next multiple of 4.
    /// So for instance asking for just 1 bit password will result in 4 bits to be emitted, in the form of a single hex character.
    #[must_use]
    pub fn bits(self, bits: usize) -> Self {
        Self {
            command_args: self.command_args.arg(bits),
        }
    }
}

impl IntoArgs for AclGenPassOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Options for the [`acl_log`](crate::ServerCommands::acl_log) command
#[derive(Default)]
pub struct AclLogOptions {
    command_args: CommandArgs,
}

impl AclLogOptions {
    /// This optional argument specifies how many entries to show.
    /// By default up to ten failures are returned.
    #[must_use]
    pub fn count(self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg(count),
        }
    }

    /// The special RESET argument clears the log.
    #[must_use]
    pub fn reset(self) -> Self {
        Self {
            command_args: self.command_args.arg("RESET"),
        }
    }
}

impl IntoArgs for AclLogOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Command info result for the [`command`](crate::ServerCommands::command) command.
#[derive(Debug)]
pub struct CommandInfo {
    /// This is the command's name in lowercase.
    pub name: String,
    /// Arity is the number of arguments a command expects. It follows a simple pattern:
    /// - A positive integer means a fixed number of arguments.
    /// - A negative integer means a minimal number of arguments.
    pub arity: isize,
    /// Command flags are an array.
    /// See [COMMAND documentation](https://redis.io/commands/command/) for the list of flags
    pub flags: Vec<String>,
    /// The position of the command's first key name argument.
    /// For most commands, the first key's position is 1. Position 0 is always the command name itself.
    pub first_key: usize,
    /// The position of the command's last key name argument.
    pub last_key: isize,
    /// The step, or increment, between the first key and the position of the next key.
    pub step: usize,
    /// [From Redis 6.0] This is an array of simple strings that are the ACL categories to which the command belongs.
    pub acl_categories: Vec<String>,
    /// [From Redis 7.0] Helpful information about the command. To be used by clients/proxies.
    /// See [<https://redis.io/docs/reference/command-tips/>](https://redis.io/docs/reference/command-tips/)
    pub command_tips: Vec<CommandTip>,
    /// [From Redis 7.0] This is an array consisting of the command's key specifications.
    /// See [<https://redis.io/docs/reference/key-specs/>](https://redis.io/docs/reference/key-specs/)
    pub key_specifications: Vec<KeySpecification>,
    pub sub_commands: Vec<CommandInfo>,
}

impl FromValue for CommandInfo {
    fn from_value(value: Value) -> Result<Self> {
        let values: Vec<Value> = value.into()?;
        let mut iter = values.into_iter();

        match (
            iter.next(),
            iter.next(),
            iter.next(),
            iter.next(),
            iter.next(),
            iter.next(),
            iter.next(),
            iter.next(),
            iter.next(),
            iter.next(),
        ) {
            (
                Some(name),
                Some(arity),
                Some(flags),
                Some(first_key),
                Some(last_key),
                Some(step),
                Some(acl_categories),
                Some(command_tips),
                Some(key_specifications),
                Some(sub_commands),
            ) => Ok(Self {
                name: name.into()?,
                arity: arity.into()?,
                flags: flags.into()?,
                first_key: first_key.into()?,
                last_key: last_key.into()?,
                step: step.into()?,
                acl_categories: acl_categories.into()?,
                command_tips: command_tips.into()?,
                key_specifications: key_specifications.into()?,
                sub_commands: sub_commands.into()?,
            }),
            (
                Some(name),
                Some(arity),
                Some(flags),
                Some(first_key),
                Some(last_key),
                Some(step),
                Some(acl_categories),
                None,
                None,
                None,
            ) => Ok(Self {
                name: name.into()?,
                arity: arity.into()?,
                flags: flags.into()?,
                first_key: first_key.into()?,
                last_key: last_key.into()?,
                step: step.into()?,
                acl_categories: acl_categories.into()?,
                command_tips: Vec::new(),
                key_specifications: Vec::new(),
                sub_commands: Vec::new(),
            }),
            (
                Some(name),
                Some(arity),
                Some(flags),
                Some(first_key),
                Some(last_key),
                Some(step),
                None,
                None,
                None,
                None,
            ) => Ok(Self {
                name: name.into()?,
                arity: arity.into()?,
                flags: flags.into()?,
                first_key: first_key.into()?,
                last_key: last_key.into()?,
                step: step.into()?,
                acl_categories: Vec::new(),
                command_tips: Vec::new(),
                key_specifications: Vec::new(),
                sub_commands: Vec::new(),
            }),
            _ => Err(Error::Parse(
                "Cannot parse CommandInfo from result".to_owned(),
            )),
        }

        //let (name, arity, flags, first_key, last_key, step, acl_categories, command_tips, key_specifications, sub_commands)
    }
}

/// Get additional information about a command
/// See <https://redis.io/docs/reference/command-tips/>
#[derive(Debug)]
pub enum CommandTip {
    NonDeterministricOutput,
    NonDeterministricOutputOrder,
    RequestPolicy(RequestPolicy),
    ResponsePolicy(ResponsePolicy),
}

impl FromValue for CommandTip {
    fn from_value(value: Value) -> Result<Self> {
        let tip: String = value.into()?;
        match tip.as_str() {
            "nondeterministic_output" => Ok(CommandTip::NonDeterministricOutput),
            "nondeterministic_output_order" => Ok(CommandTip::NonDeterministricOutputOrder),
            _ => {
                let mut parts = tip.split(':');
                match (parts.next(), parts.next(), parts.next()) {
                    (Some("request_policy"), Some(policy), None) => {
                        Ok(CommandTip::RequestPolicy(RequestPolicy::from_str(policy)?))
                    }
                    (Some("response_policy"), Some(policy), None) => Ok(
                        CommandTip::ResponsePolicy(ResponsePolicy::from_str(policy)?),
                    ),
                    _ => Err(Error::Parse(
                        "Cannot parse CommandTip from result".to_owned(),
                    )),
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum RequestPolicy {
    AllNodes,
    AllShards,
    MultiShard,
    Special,
}

impl RequestPolicy {
    pub fn from_str(str: &str) -> Result<Self> {
        match str {
            "all_nodes" => Ok(RequestPolicy::AllNodes),
            "all_shards" => Ok(RequestPolicy::AllShards),
            "multi_shard" => Ok(RequestPolicy::MultiShard),
            "special" => Ok(RequestPolicy::Special),
            _ => Err(Error::Parse(
                "Cannot parse RequestPolicy from result".to_owned(),
            )),
        }
    }
}

#[derive(Debug)]
pub enum ResponsePolicy {
    OneSucceeded,
    AllSucceeded,
    AggLogicalAnd,
    AggLogicalOr,
    AggMin,
    AggMax,
    AggSum,
    Special,
}

impl ResponsePolicy {
    pub fn from_str(str: &str) -> Result<Self> {
        match str {
            "one_succeeded" => Ok(ResponsePolicy::OneSucceeded),
            "all_succeeded" => Ok(ResponsePolicy::AllSucceeded),
            "agg_logical_and" => Ok(ResponsePolicy::AggLogicalAnd),
            "agg_logical_or" => Ok(ResponsePolicy::AggLogicalOr),
            "agg_min" => Ok(ResponsePolicy::AggMin),
            "agg_max" => Ok(ResponsePolicy::AggMax),
            "agg_sum" => Ok(ResponsePolicy::AggSum),
            "special" => Ok(ResponsePolicy::Special),
            _ => Err(Error::Parse(
                "Cannot parse ResponsePolicy from result".to_owned(),
            )),
        }
    }
}

/// Key specifications of a command for the [`command`](crate::ServerCommands::command) command.
#[derive(Debug)]
pub struct KeySpecification {
    pub begin_search: BeginSearch,
    pub find_keys: FindKeys,
    pub flags: Vec<String>,
    pub notes: String,
}

impl FromValue for KeySpecification {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: HashMap<String, Value> = value.into()?;

        let notes: String = match values.remove("notes") {
            Some(notes) => notes.into()?,
            None => "".to_owned(),
        };

        Ok(Self {
            begin_search: values.remove_with_result("begin_search")?.into()?,
            find_keys: values.remove_with_result("find_keys")?.into()?,
            flags: values.remove_with_result("flags")?.into()?,
            notes,
        })
    }
}

#[derive(Debug)]
pub enum BeginSearch {
    Index(usize),
    Keyword { keyword: String, start_from: isize },
    Unknown,
}

impl FromValue for BeginSearch {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: HashMap<String, Value> = value.into()?;

        let type_: String = values.remove_with_result("type")?.into()?;
        match type_.as_str() {
            "index" => {
                let mut spec: HashMap<String, Value> = values.remove_with_result("spec")?.into()?;
                Ok(BeginSearch::Index(
                    spec.remove_with_result("index")?.into()?,
                ))
            }
            "keyword" => {
                let mut spec: HashMap<String, Value> = values.remove_with_result("spec")?.into()?;
                Ok(BeginSearch::Keyword {
                    keyword: spec.remove_with_result("keyword")?.into()?,
                    start_from: spec.remove_with_result("startfrom")?.into()?,
                })
            }
            "unknown" => Ok(BeginSearch::Unknown),
            _ => Err(Error::Parse(
                "Cannot parse BeginSearch from result".to_owned(),
            )),
        }
    }
}

#[derive(Debug)]
pub enum FindKeys {
    Range {
        last_key: isize,
        key_step: usize,
        limit: isize,
    },
    KeyEnum {
        key_num_idx: isize,
        first_key: isize,
        key_step: usize,
    },
    Unknown,
}

impl FromValue for FindKeys {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: HashMap<String, Value> = value.into()?;

        let type_: String = values.remove_with_result("type")?.into()?;
        match type_.as_str() {
            "range" => {
                let mut spec: HashMap<String, Value> = values.remove_with_result("spec")?.into()?;
                Ok(FindKeys::Range {
                    last_key: spec.remove_with_result("lastkey")?.into()?,
                    key_step: spec.remove_with_result("keystep")?.into()?,
                    limit: spec.remove_with_result("limit")?.into()?,
                })
            }
            "keynum" => {
                let mut spec: HashMap<String, Value> = values.remove_with_result("spec")?.into()?;
                Ok(FindKeys::KeyEnum {
                    key_num_idx: spec.remove_with_result("keynumidx")?.into()?,
                    first_key: spec.remove_with_result("firstkey")?.into()?,
                    key_step: spec.remove_with_result("keystep")?.into()?,
                })
            }
            "unknown" => Ok(FindKeys::Unknown),
            _ => Err(Error::Parse(
                "Cannot parse BeginSearch from result".to_owned(),
            )),
        }
    }
}

/// Command doc result for the [`command_docs`](crate::ServerCommands::command_docs) command
#[derive(Debug, Default)]
pub struct CommandDoc {
    /// short command description.
    pub summary: String,
    /// the Redis version that added the command (or for module commands, the module version).
    pub since: String,
    /// he functional group to which the command belongs.
    pub group: String,
    /// a short explanation about the command's time complexity.
    pub complexity: String,
    /// an array of documentation flags. Possible values are:
    /// - `deprecated`: the command is deprecated.
    /// - `syscmd`: a system command that isn't meant to be called by users.
    pub doc_flags: Vec<CommandDocFlag>,
    /// the Redis version that deprecated the command (or for module commands, the module version).
    pub deprecated_since: String,
    /// the alternative for a deprecated command.
    pub replaced_by: String,
    /// an array of historical notes describing changes to the command's behavior or arguments.
    pub history: Vec<HistoricalNote>,
    /// an array of [`command arguments`](https://redis.io/docs/reference/command-arguments/)
    pub arguments: Vec<CommandArgument>,
}

impl FromValue for CommandDoc {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: HashMap<String, Value> = value.into()?;

        Ok(Self {
            summary: values.remove_with_result("summary")?.into()?,
            since: values.remove_with_result("since")?.into()?,
            group: values.remove_with_result("group")?.into()?,
            complexity: values.remove_with_result("complexity")?.into()?,
            doc_flags: values.remove_or_default("doc_flags").into()?,
            deprecated_since: values.remove_or_default("deprecated_since").into()?,
            replaced_by: values.remove_or_default("replaced_by").into()?,
            history: values.remove_or_default("history").into()?,
            arguments: values.remove_with_result("arguments")?.into()?,
        })
    }
}

/// Command documenation flag
#[derive(Debug)]
pub enum CommandDocFlag {
    /// the command is deprecated.
    Deprecated,
    /// a system command that isn't meant to be called by users.
    SystemCommand,
}

impl FromValue for CommandDocFlag {
    fn from_value(value: Value) -> Result<Self> {
        let f: String = value.into()?;

        match f.as_str() {
            "deprecated" => Ok(CommandDocFlag::Deprecated),
            "syscmd" => Ok(CommandDocFlag::SystemCommand),
            _ => Err(Error::Parse(
                "Cannot parse CommandDocFlag from result".to_owned(),
            )),
        }
    }
}

#[derive(Debug)]
pub struct HistoricalNote {
    pub version: String,
    pub description: String,
}

impl FromValue for HistoricalNote {
    fn from_value(value: Value) -> Result<Self> {
        let (version, description): (String, String) = value.into()?;

        Ok(Self {
            version,
            description,
        })
    }
}

/// [`command argument`](https://redis.io/docs/reference/command-arguments/)
#[derive(Debug)]
pub struct CommandArgument {
    ///  the argument's name, always present.
    pub name: String,
    /// the argument's display string, present in arguments that have a displayable representation
    pub display_text: String,
    ///  the argument's type, always present.
    pub type_: CommandArgumentType,
    /// this value is available for every argument of the `key` type.
    /// t is a 0-based index of the specification in the command's [`key specifications`](https://redis.io/topics/key-specs)
    /// that corresponds to the argument.
    pub key_spec_index: usize,
    /// a constant literal that precedes the argument (user input) itself.
    pub token: String,
    /// a short description of the argument.
    pub summary: String,
    /// the debut Redis version of the argument (or for module commands, the module version).
    pub since: String,
    /// the Redis version that deprecated the command (or for module commands, the module version).
    pub deprecated_since: String,
    /// an array of argument flags.
    pub flags: Vec<ArgumentFlag>,
    /// the argument's value.
    pub value: Vec<String>,
}

impl FromValue for CommandArgument {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: HashMap<String, Value> = value.into()?;

        Ok(Self {
            name: values.remove_with_result("name")?.into()?,
            display_text: values.remove_or_default("display_text").into()?,
            type_: values.remove_with_result("type")?.into()?,
            key_spec_index: values.remove_or_default("key_spec_index").into()?,
            token: values.remove_or_default("token").into()?,
            summary: values.remove_or_default("summary").into()?,
            since: values.remove_or_default("since").into()?,
            deprecated_since: values.remove_or_default("deprecated_since").into()?,
            flags: values.remove_or_default("flags").into()?,
            value: match values.remove_or_default("value") {
                value @ Value::BulkString(_) => vec![value.into()?],
                value @ Value::Array(_) => value.into()?,
                _ => {
                    return Err(Error::Parse(
                        "Cannot parse CommandArgument from result".to_owned(),
                    ))
                }
            },
        })
    }
}

/// An argument must have one of the following types:
#[derive(Debug)]
pub enum CommandArgumentType {
    /// a string argument.
    String,
    /// an integer argument.
    Integer,
    /// a double-precision argument.
    Double,
    /// a string that represents the name of a key.
    Key,
    /// a string that represents a glob-like pattern.
    Pattern,
    /// an integer that represents a Unix timestamp.
    UnixTime,
    /// a token, i.e. a reserved keyword, which may or may not be provided.
    /// Not to be confused with free-text user input.
    PureToken,
    /// the argument is a container for nested arguments.
    /// This type enables choice among several nested arguments
    OneOf,
    /// the argument is a container for nested arguments.
    /// This type enables grouping arguments and applying a property (such as optional) to all
    Block,
}

impl FromValue for CommandArgumentType {
    fn from_value(value: Value) -> Result<Self> {
        let t: String = value.into()?;

        match t.as_str() {
            "string" => Ok(CommandArgumentType::String),
            "integer" => Ok(CommandArgumentType::Integer),
            "double" => Ok(CommandArgumentType::Double),
            "key" => Ok(CommandArgumentType::Key),
            "pattern" => Ok(CommandArgumentType::Pattern),
            "unix-time" => Ok(CommandArgumentType::UnixTime),
            "pure-token" => Ok(CommandArgumentType::PureToken),
            "oneof" => Ok(CommandArgumentType::OneOf),
            "block" => Ok(CommandArgumentType::Block),
            _ => Err(Error::Parse(
                "Cannot parse CommandArgumentType from result".to_owned(),
            )),
        }
    }
}

/// Flag for a command argument
#[derive(Debug)]
pub enum ArgumentFlag {
    /// denotes that the argument is optional (for example, the GET clause of the SET command).
    Optional,
    /// denotes that the argument may be repeated (such as the key argument of DEL).
    Multiple,
    ///  denotes the possible repetition of the argument with its preceding token (see SORT's GET pattern clause).
    MultipleToken,
}

impl FromValue for ArgumentFlag {
    fn from_value(value: Value) -> Result<Self> {
        let f: String = value.into()?;

        match f.as_str() {
            "optional" => Ok(ArgumentFlag::Optional),
            "multiple" => Ok(ArgumentFlag::Multiple),
            "multiple-token" => Ok(ArgumentFlag::MultipleToken),
            _ => Err(Error::Parse(
                "Cannot parse ArgumentFlag from result".to_owned(),
            )),
        }
    }
}

/// Options for the [`command_list`](crate::ServerCommands::command_list) command.
#[derive(Default)]
pub struct CommandListOptions {
    command_args: CommandArgs,
}

impl CommandListOptions {
    /// get the commands that belong to the module specified by `module-name`.
    #[must_use]
    pub fn filter_by_module_name<M: Into<BulkString>>(self, module_name: M) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("FILTERBY")
                .arg("MODULE")
                .arg(module_name),
        }
    }

    /// get the commands in the [`ACL category`](https://redis.io/docs/manual/security/acl/#command-categories) specified by `category`.
    #[must_use]
    pub fn filter_by_acl_category<C: Into<BulkString>>(self, category: C) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("FILTERBY")
                .arg("ACLCAT")
                .arg(category),
        }
    }

    /// get the commands that match the given glob-like `pattern`.
    #[must_use]
    pub fn filter_by_pattern<P: Into<BulkString>>(self, pattern: P) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("FILTERBY")
                .arg("PATTERN")
                .arg(pattern),
        }
    }
}

impl IntoArgs for CommandListOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Options for the [`failover`](crate::ServerCommands::failover) command.
#[derive(Default)]
pub struct FailOverOptions {
    command_args: CommandArgs,
}

impl FailOverOptions {
    /// This option allows designating a specific replica, by its host and port, to failover to.
    #[must_use]
    pub fn to<H: Into<BulkString>>(self, host: H, port: u16) -> Self {
        Self {
            command_args: self.command_args.arg("TO").arg(host).arg(port),
        }
    }

    /// This option allows specifying a maximum time a master will wait in the waiting-for-sync state
    /// before aborting the failover attempt and rolling back.
    #[must_use]
    pub fn timeout(self, milliseconds: u64) -> Self {
        Self {
            command_args: self.command_args.arg("TIMEOUT").arg(milliseconds),
        }
    }

    /// If both the [`timeout`](crate::FailOverOptions::timeout) and [`to`](crate::FailOverOptions::to) options are set,
    /// the force flag can also be used to designate that that once the timeout has elapsed,
    /// the master should failover to the target replica instead of rolling back.
    #[must_use]
    pub fn force(self) -> Self {
        Self {
            command_args: self.command_args.arg("FORCE"),
        }
    }

    /// This command will abort an ongoing failover and return the master to its normal state. 
    #[must_use]
    pub fn abort(self) -> Self {
        Self {
            command_args: self.command_args.arg("ABORT"),
        }
    }
}

impl IntoArgs for FailOverOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Section for the [`info`](crate::ServerCommands::info) command.
pub enum InfoSection {
    Server,
    Clients,
    Memory,
    Persistence,
    Stats,
    Replication,
    Cpu,
    Commandstats,
    Latencystats,
    Cluster,
    Keyspace,
    Modules,
    Errorstats,
    All,
    Default,
    Everything
}

impl From<InfoSection> for BulkString {
    fn from(s: InfoSection) -> Self {
        match s {
            InfoSection::Server => BulkString::Str("server"),
            InfoSection::Clients => BulkString::Str("clients"),
            InfoSection::Memory => BulkString::Str("memory"),
            InfoSection::Persistence => BulkString::Str("persistence"),
            InfoSection::Stats => BulkString::Str("stats"),
            InfoSection::Replication => BulkString::Str("replication"),
            InfoSection::Cpu => BulkString::Str("cpu"),
            InfoSection::Commandstats => BulkString::Str("commandstats"),
            InfoSection::Latencystats => BulkString::Str("latencystats"),
            InfoSection::Cluster => BulkString::Str("cluster"),
            InfoSection::Keyspace => BulkString::Str("keyspace"),
            InfoSection::Modules => BulkString::Str("modules"),
            InfoSection::Errorstats => BulkString::Str("errorstats"),
            InfoSection::All => BulkString::Str("all"),
            InfoSection::Default => BulkString::Str("default"),
            InfoSection::Everything => BulkString::Str("everything"),
        }
    }
}
