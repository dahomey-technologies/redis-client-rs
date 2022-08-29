use crate::{cmd, tests::get_default_addr, ConnectionMultiplexer, Error, Result};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn unknown_command() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    let result = database.send(cmd("UNKNOWN").arg("arg")).await;

    assert!(
        matches!(result, Err(Error::Redis(e)) if e.starts_with("ERR unknown command 'UNKNOWN'"))
    );

    Ok(())
}

// #[cfg_attr(feature = "tokio-runtime", tokio::test)]
// #[cfg_attr(feature = "async-std-runtime", async_std::test)]
// #[serial]
// async fn network_error() -> Result<()> {
//     let connection = ConnectionMultiplexer::connect().await?;
//     let database = connection.get_default_database();

//     for i in 1..1000 {
//         let key = format!("key{}", i);
//         let value = format!("value{}", i);
//         database.set(key, value).await?;
//     }

//     for i in 1..1000 {
//         let key = format!("key{}", i);
//         let result: Result<String> = database.get(key.clone()).await;
//         println!("test key: {:?}, value: {:?}", key, result);
//         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
//     }

//     Ok(())
// }

// #[cfg_attr(feature = "tokio-runtime", tokio::test)]
// #[cfg_attr(feature = "async-std-runtime", async_std::test)]
// #[serial]
// async fn network_error_stress_test() -> Result<()> {
//     let connection = ConnectionMultiplexer::connect().await?;
//     let database = connection.get_default_database();

//     for i in 1..1000 {
//         let key = format!("key{}", i);
//         let value = format!("value{}", i);
//         database.set(key, value).await?;
//     }

//     use rand::Rng;

//     let tasks: Vec<_> = (1..8)
//         .into_iter()
//         .map(|_| {
//             let db = database.clone();
//             tokio::spawn(async move {
//                 for _ in 1..10000 {
//                     let i = rand::thread_rng().gen_range(1..1000);
//                     let key = format!("key{}", i);
//                     let result: Result<String> = db.get(key.clone()).await;
//                     println!("test key: {:?}, value: {:?}", key, result);
//                     //tokio::time::sleep(std::time::Duration::from_secs(1)).await;
//                 }
//             })
//         })
//         .collect();

//     future::join_all(tasks).await;

//     Ok(())
// }
