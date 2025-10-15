use rabbitmq_http_client::blocking_api::Client;
use rabbitmq_http_client::responses::QueueInfo;
use regex::Regex;

pub struct Queue{
    pub name: String,
    pub messages: u64,
    consumer_count: u16,
}

pub struct CollectedObjects{
    pub delete_queues: Vec<Queue>,
    pub purge_queues: Vec<Queue>,
    pub delete_exchanges: Vec<String>,
}

impl Queue {
    pub fn from(info: QueueInfo) -> Queue {
        Queue {
            name: info.name,
            messages: info.message_count,
            consumer_count: info.consumer_count,
        }
    }
}

pub fn collect_objects(
    rc: &Client<&str, &str, &str>,
    vhost: &str,
    filter: &str,
    queues: bool,
    exchanges: bool,
) -> Result<CollectedObjects, Box<dyn std::error::Error>> {
    let mut delete_queues = Vec::new();
    let mut purge_queues = Vec::new();
    let mut delete_exchanges = Vec::new();

    let re = Regex::new(filter)?;

    for queue in rc.list_queues_in(vhost)? {
        if !re.is_match(&queue.name) {
            continue;
        }

        if queue.exclusive {
            println!("🚫  Skipping exclusive queue {}", queue.name);
            continue;
        }

        if queues {
            delete_queues.push(Queue::from(queue));
        } else if queue.message_count > 0 {
            purge_queues.push(Queue::from(queue));
        }
    }

    if exchanges {
        let skip_exchanges = vec![
            "",
            "amq.direct",
            "amq.fanout",
            "amq.topic",
            "amq.headers",
            "amq.match",
            "amq.rabbitmq.trace",
            "(AMQP default)",
        ];

        for exchange in rc.list_exchanges_in(vhost)? {
            if !re.is_match(&exchange.name)|| skip_exchanges.contains(&exchange.name.as_str())
            {
                continue;
            }

            delete_exchanges.push(exchange.name);
        }
    }

    Ok(CollectedObjects {
        delete_queues,
        purge_queues,
        delete_exchanges,
    })
}