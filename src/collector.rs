use rabbitmq_http_client::blocking_api::Client;
use regex::Regex;

pub struct Queue{
    pub name: String,
    pub messages: u64,
}

pub struct CollectedObjects{
    pub delete_queues: Vec<Queue>,
    pub purge_queues: Vec<Queue>,
    pub delete_exchanges: Vec<String>,
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

    for queue in rc.list_queues()? {
        if vhost != queue.vhost || !re.is_match(&queue.name) {
            continue;
        }

        if queue.exclusive {
            println!("🚫  Skipping exclusive queue {}", queue.name);
            continue;
        }

        if queues {
            delete_queues.push(Queue {
                name: queue.name,
                messages: queue.message_count,
            });
        } else if queue.message_count > 0 {
            purge_queues.push(Queue {
                name: queue.name,
                messages: queue.message_count,
            });
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

        let exchanges = rc.list_exchanges()?;
        for exchange in exchanges {
            if vhost != exchange.vhost
                || !re.is_match(&exchange.name)
                || skip_exchanges.contains(&exchange.name.as_str())
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