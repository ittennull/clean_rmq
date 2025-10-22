mod common;

use crate::common::{create_args, wait_for_0_messages, TestClient};
use clean_rmq::{Action, Args, DeleteOptions};
use std::error::Error;

pub type TestingResult = Result<(), Box<dyn Error>>;

#[test]
fn delete_in_dry_run_mode_doesnt_change_anything() -> TestingResult {
    let client = TestClient::new()?;
    client.create_exchange("e1")?; // unbound exchange
    client.create_queue("q1")?; // unbound queue
    client.create_exchange("e2")?; // exchange with bound queue
    client.create_connected_queue("q2", "e2")?; // bound queue
    client.publish_message_and_wait_delivery_in("e2", "q2")?; // queue2 is not empty
    client.create_exchange("e3")?; // exchange with bound queue
    client.create_connected_queue("q3", "e3")?; // empty bound queue

    let args = Args {
        action: delete_action(|options| {
            options.queues = true;
            options.exchanges = true;
        }),
        ..create_args(&client, true)
    };
    clean_rmq::run(args)?;

    let exchanges = client.list_exchanges()?;
    let queues = client.list_queues()?;
    assert!(exchanges.contains(&"e1".to_string()));
    assert!(exchanges.contains(&"e2".to_string()));
    assert!(exchanges.contains(&"e3".to_string()));
    assert!(queues.contains(&"q1".to_string()));
    assert!(queues.contains(&"q2".to_string()));
    assert!(queues.contains(&"q3".to_string()));

    Ok(())
}

#[test]
fn purge_in_dry_run_mode_doesnt_change_anything() -> TestingResult {
    let client = TestClient::new()?;
    client.create_exchange("e1")?;
    client.create_connected_queue("empty_queue", "e1")?;
    client.create_exchange("e2")?; // exchange with bound queue
    client.create_connected_queue("with_message", "e2")?; // bound queue
    client.publish_message_and_wait_delivery_in("e2", "with_message")?; // queue2 is not empty

    let args = Args {
        action: Some(Action::Purge {
            queue_filter: ".+".to_string(),
            exclude_queue_filter: vec![],
        }),
        ..create_args(&client, true)
    };
    clean_rmq::run(args)?;

    let exchanges = client.list_exchanges()?;
    let queues = client.list_queues()?;
    assert!(exchanges.contains(&"e1".to_string()));
    assert!(exchanges.contains(&"e2".to_string()));
    assert!(queues.contains(&"empty_queue".to_string()));
    assert!(queues.contains(&"with_message".to_string()));
    assert_eq!(1, client.get_number_of_messages("with_message")?);

    Ok(())
}

#[test]
fn purge_with_filter() -> TestingResult {
    let client = TestClient::new()?;

    let setup = |exchange: &str, queue: &str| -> TestingResult {
        client.create_exchange(exchange)?;
        client.create_connected_queue(queue, exchange)?;
        client.publish_message_and_wait_delivery_in(exchange, queue)?;
        Ok(())
    };

    setup("e1", "queue1")?;
    setup("e2", "queue_error")?;
    setup("e3", "queue_special_error")?;

    let args = Args {
        action: Some(Action::Purge {
            queue_filter: ".+_error".to_string(),
            exclude_queue_filter: vec![".*special.*".to_string()],
        }),
        ..create_args(&client, false)
    };
    clean_rmq::run(args)?;

    wait_for_0_messages(&client, "queue_error")?;
    assert_eq!(0, client.get_number_of_messages("queue_error")?); // hits include filter, so it is purged
    assert_eq!(1, client.get_number_of_messages("queue1")?); // does not hit include filter, so it is not purged
    assert_eq!(1, client.get_number_of_messages("queue_special_error")?); // hits include and exclude filter, so it is not purged

    Ok(())
}

#[test]
fn delete_all_exchanges() -> TestingResult {
    let client = TestClient::new()?;
    client.create_exchange("one")?;
    client.create_exchange("two")?;

    let args = Args {
        action: delete_action(|options| {
            options.exchanges = true;
        }),
        ..create_args(&client, false)
    };
    clean_rmq::run(args)?;

    let exchanges = client.list_exchanges()?;
    assert!(!exchanges.contains(&"one".to_string()));
    assert!(!exchanges.contains(&"two".to_string()));

    Ok(())
}

#[test]
fn delete_only_exchanges_without_destination() -> TestingResult {
    let client = TestClient::new()?;
    client.create_exchange("one")?;
    client.create_exchange("two")?;
    client.create_connected_queue("queue", "two")?;

    let args = Args {
        action: delete_action(|options| {
            options.exchanges = true;
            options.exchanges_without_destination = true;
        }),
        ..create_args(&client, false)
    };
    clean_rmq::run(args)?;

    let exchanges = client.list_exchanges()?;
    assert!(!exchanges.contains(&"one".to_string()));
    assert!(exchanges.contains(&"two".to_string()));

    let queues = client.list_queues()?;
    assert!(queues.contains(&"queue".to_string())); // Ensure the connected queue still exists

    Ok(())
}

#[test]
fn delete_only_exchanges_without_destination_and_complex_topology() -> TestingResult {
    let client = TestClient::new()?;
    create_complex_topology(&client)?;

    let args = Args {
        action: delete_action(|options| {
            options.exchanges = true;
            options.exchanges_without_destination = true;
        }),
        ..create_args(&client, false)
    };
    clean_rmq::run(args)?;

    let exchanges = client.list_exchanges()?;
    assert!(exchanges.iter().all(|x| !x.starts_with("ex")));

    Ok(())
}

#[test]
fn delete_only_exchanges_without_destination_and_complex_topology_and_bound_ex6() -> TestingResult {
    let client = TestClient::new()?;
    create_complex_topology(&client)?;
    client.create_connected_queue("queue1", "ex6")?;

    let args = Args {
        action: delete_action(|options| {
            options.exchanges = true;
            options.exchanges_without_destination = true;
        }),
        ..create_args(&client, false)
    };
    clean_rmq::run(args)?;

    let exchanges = client.list_exchanges()?;
    assert!(exchanges.contains(&"ex1".to_string()));
    assert!(!exchanges.contains(&"ex2".to_string()));
    assert!(!exchanges.contains(&"ex3".to_string()));
    assert!(exchanges.contains(&"ex4".to_string()));
    assert!(exchanges.contains(&"ex5".to_string()));
    assert!(exchanges.contains(&"ex6".to_string()));

    Ok(())
}

#[test]
fn delete_only_exchanges_without_destination_and_complex_topology_and_bound_ex3() -> TestingResult {
    let client = TestClient::new()?;
    create_complex_topology(&client)?;
    client.create_connected_queue("queue1", "ex3")?;

    let args = Args {
        action: delete_action(|options| {
            options.exchanges = true;
            options.exchanges_without_destination = true;
        }),
        ..create_args(&client, false)
    };
    clean_rmq::run(args)?;

    let exchanges = client.list_exchanges()?;
    assert!(exchanges.contains(&"ex1".to_string()));
    assert!(exchanges.contains(&"ex2".to_string()));
    assert!(exchanges.contains(&"ex3".to_string()));
    assert!(exchanges.contains(&"ex4".to_string()));
    assert!(!exchanges.contains(&"ex5".to_string()));
    assert!(!exchanges.contains(&"ex6".to_string()));

    Ok(())
}

#[test]
fn delete_only_exchanges_without_destination_and_complex_topology_and_bound_ex5() -> TestingResult {
    let client = TestClient::new()?;
    create_complex_topology(&client)?;
    client.create_connected_queue("queue1", "ex5")?;

    let args = Args {
        action: delete_action(|options| {
            options.exchanges = true;
            options.exchanges_without_destination = true;
        }),
        ..create_args(&client, false)
    };
    clean_rmq::run(args)?;

    let exchanges = client.list_exchanges()?;
    assert!(exchanges.contains(&"ex1".to_string()));
    assert!(!exchanges.contains(&"ex2".to_string()));
    assert!(!exchanges.contains(&"ex3".to_string()));
    assert!(exchanges.contains(&"ex4".to_string()));
    assert!(exchanges.contains(&"ex5".to_string()));
    assert!(!exchanges.contains(&"ex6".to_string()));

    Ok(())
}

#[test]
fn delete_only_exchanges_without_destination_and_complex_topology_and_bound_ex3_and_ex6()
-> TestingResult {
    let client = TestClient::new()?;
    create_complex_topology(&client)?;
    client.create_connected_queue("queue1", "ex3")?;
    client.create_connected_queue("queue2", "ex6")?;

    let args = Args {
        action: delete_action(|options| {
            options.exchanges = true;
            options.exchanges_without_destination = true;
        }),
        ..create_args(&client, false)
    };
    clean_rmq::run(args)?;

    let exchanges = client.list_exchanges()?;
    assert!(exchanges.contains(&"ex1".to_string()));
    assert!(exchanges.contains(&"ex2".to_string()));
    assert!(exchanges.contains(&"ex3".to_string()));
    assert!(exchanges.contains(&"ex4".to_string()));
    assert!(exchanges.contains(&"ex5".to_string()));
    assert!(exchanges.contains(&"ex6".to_string()));

    Ok(())
}

#[test]
fn delete_only_exchanges_without_destination_and_complex_topology_and_bound_ex5_but_queue_is_also_deleted()
-> TestingResult {
    let client = TestClient::new()?;
    create_complex_topology(&client)?;
    client.create_connected_queue("queue1", "ex5")?;

    let args = Args {
        action: delete_action(|options| {
            options.queues = true;
            options.exchanges = true;
            options.exchanges_without_destination = true;
        }),
        ..create_args(&client, false)
    };
    clean_rmq::run(args)?;

    let exchanges = client.list_exchanges()?;
    assert!(!exchanges.contains(&"ex1".to_string()));
    assert!(!exchanges.contains(&"ex2".to_string()));
    assert!(!exchanges.contains(&"ex3".to_string()));
    assert!(!exchanges.contains(&"ex4".to_string()));
    assert!(!exchanges.contains(&"ex5".to_string()));
    assert!(!exchanges.contains(&"ex6".to_string()));

    Ok(())
}

#[test]
fn delete_queues() -> TestingResult {
    let client = TestClient::new()?;
    client.create_queue("one")?;
    client.create_exchange("two")?;
    client.create_connected_queue("two", "two")?;

    let args = Args {
        action: delete_action(|options| {
            options.queues = true;
        }),
        ..create_args(&client, false)
    };
    clean_rmq::run(args)?;

    let exchanges = client.list_exchanges()?;
    assert!(exchanges.contains(&"two".to_string()));

    let queues = client.list_queues()?;
    assert!(!queues.contains(&"one".to_string()));
    assert!(!queues.contains(&"two".to_string()));

    Ok(())
}

#[test]
fn delete_queues_with_filter() -> TestingResult {
    let client = TestClient::new()?;
    client.create_queue("one")?;
    client.create_exchange("two")?;
    client.create_connected_queue("two", "two")?;
    client.create_queue("hello")?;

    let args = Args {
        action: delete_action(|options| {
            options.queues = true;
            options.queue_filter = ".o".to_string(); // ends with 'o'
            options.exclude_queue_filter = vec!["hell.*".to_string()]; // does not start with 't'
        }),
        ..create_args(&client, false)
    };
    clean_rmq::run(args)?;

    let queues = client.list_queues()?;
    assert!(queues.contains(&"one".to_string())); // does not hit include filter, so it is not deleted
    assert!(!queues.contains(&"two".to_string())); // hits include filter, so it is deleted
    assert!(queues.contains(&"hello".to_string())); // hits exclude filter, so it is not deleted

    Ok(())
}

fn delete_action(f: fn(&mut DeleteOptions)) -> Option<Action> {
    let mut options = DeleteOptions {
        queues: false,
        queues_without_consumers: false,
        queue_filter: "".to_string(),
        exclude_queue_filter: vec![],
        exchanges: false,
        exchanges_without_destination: false,
    };
    f(&mut options);
    Some(Action::Delete(options))
}

/// create this topology:
/// ex1  →  ex2  →  ex3
///   🡾            🡽
///     ex4 ------
///       🡾
///         ex5  →  ex6
fn create_complex_topology(client: &TestClient) -> TestingResult {
    client.create_exchange("ex1")?;
    client.create_exchange("ex2")?;
    client.create_exchange("ex3")?;
    client.create_exchange("ex4")?;
    client.create_exchange("ex5")?;
    client.create_exchange("ex6")?;

    client.bind_exchange("ex1", "ex2")?;
    client.bind_exchange("ex2", "ex3")?;
    client.bind_exchange("ex1", "ex4")?;
    client.bind_exchange("ex4", "ex3")?;
    client.bind_exchange("ex4", "ex5")?;
    client.bind_exchange("ex5", "ex6")?;

    Ok(())
}
