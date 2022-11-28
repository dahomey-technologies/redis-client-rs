use crate::{
    resp::{Value},
    tests::get_test_client,
    GenericCommands, Result, SetCommands,
};
use serial_test::serial;
use std::collections::{BTreeSet, HashSet};

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn from_single_value_array() -> Result<()> {
    let mut client = get_test_client().await?;

    client.del("key").await?;

    client
        .sadd("key", ["member1", "member2", "member3"])
        .await?;

    let members: Vec<String> = client.smembers("key").await?;
    assert_eq!(3, members.len());
    assert!(members.contains(&"member1".to_owned()));
    assert!(members.contains(&"member2".to_owned()));
    assert!(members.contains(&"member3".to_owned()));

    let members: HashSet<String> = client.smembers("key").await?;
    assert_eq!(3, members.len());
    assert!(members.contains(&"member1".to_owned()));
    assert!(members.contains(&"member2".to_owned()));
    assert!(members.contains(&"member3".to_owned()));

    let members: BTreeSet<String> = client.smembers("key").await?;
    assert_eq!(3, members.len());
    assert!(members.contains(&"member1".to_owned()));
    assert!(members.contains(&"member2".to_owned()));
    assert!(members.contains(&"member3".to_owned()));

    Ok(())
}

#[test]
fn tuple() -> Result<()> {
    let value = Value::Array(Some(vec![
        Value::BulkString("first".into()),
        Value::BulkString("second".into()),
    ]));
    let result: Vec<String> = value.into()?;
    assert_eq!(2, result.len());
    assert_eq!("first".to_owned(), result[0]);
    assert_eq!("second".to_owned(), result[1]);

    let values = Value::Array(Some(vec![
        Value::BulkString("first".into()),
        Value::BulkString("second".into()),
    ]));
    let result: (String, String) = values.into()?;
    assert_eq!(("first".to_owned(), "second".to_owned()), result);

    let value = Value::Array(Some(vec![
        Value::BulkString("first".into()),
        Value::BulkString("second".into()),
        Value::BulkString("third".into()),
        Value::BulkString("fourth".into()),
    ]));
    let result: Vec<(String, String)> = value.into()?;
    assert_eq!(2, result.len());
    assert_eq!(("first".to_owned(), "second".to_owned()), result[0]);
    assert_eq!(("third".to_owned(), "fourth".to_owned()), result[1]);

    let value = Value::Array(Some(vec![
        Value::Array(Some(vec![
            Value::BulkString("first".into()),
            Value::BulkString("second".into()),
        ])),
        Value::Array(Some(vec![
            Value::BulkString("third".into()),
            Value::BulkString("fourth".into()),
        ])),
    ]));
    let result: Vec<(String, String)> = value.into()?;
    assert_eq!(2, result.len());
    assert_eq!(("first".to_owned(), "second".to_owned()), result[0]);
    assert_eq!(("third".to_owned(), "fourth".to_owned()), result[1]);

    Ok(())
}
