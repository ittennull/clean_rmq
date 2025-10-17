use rabbitmq_http_client::blocking_api::Client;
use rabbitmq_http_client::commons::BindingDestinationType;
use rabbitmq_http_client::responses::QueueInfo;
use regex::Regex;
use std::collections::{HashMap, HashSet};

pub type QueueName = String;
pub type ExchangeName = String;
pub type RmqClient<'a> = Client<&'a str, &'a str, &'a str>;

#[derive(Clone)]
pub struct Queue {
    pub name: QueueName,
    pub messages: u64,
    pub exclusive: bool,
    consumer_count: u16,
}

pub struct CollectedObjects {
    pub queues: Vec<Queue>,
    pub exchanges: Vec<ExchangeName>,
}

impl Queue {
    pub fn from(info: QueueInfo) -> Queue {
        Queue {
            name: info.name,
            messages: info.message_count,
            exclusive: info.exclusive,
            consumer_count: info.consumer_count,
        }
    }
}

pub fn collect_queues(
    rc: &RmqClient,
    vhost: &str,
    filter: &str,
) -> Result<Vec<Queue>, Box<dyn std::error::Error>> {
    let mut queues = Vec::new();
    let re = Regex::new(filter)?;

    for queue in rc.list_queues_in(vhost)? {
        if queue.message_count == 0 || !re.is_match(&queue.name) {
            continue;
        }

        queues.push(Queue::from(queue));
    }

    Ok(queues)
}

pub fn collect_objects(
    rc: &RmqClient,
    vhost: &str,
    queues: bool,
    queues_without_consumers: bool,
    queue_filter: &str,
    exchanges: bool,
    exchanges_without_destination: bool,
) -> Result<CollectedObjects, Box<dyn std::error::Error>> {
    let all_queues = rc
        .list_queues_in(vhost)?
        .into_iter()
        .map(Queue::from)
        .collect::<Vec<Queue>>();

    let queues_to_delete = if queues {
        let re = Regex::new(queue_filter)?;
        let mut queues = all_queues
            .iter()
            .filter(|x| re.is_match(&x.name))
            .cloned()
            .collect::<Vec<Queue>>();
        if queues_without_consumers {
            queues.retain(|x| x.consumer_count == 0);
        }
        queues
    } else {
        vec![]
    };

    let delete_exchanges = if exchanges {
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

        let exchanges = rc
            .list_exchanges_in(vhost)?
            .into_iter()
            .filter(|x| !skip_exchanges.contains(&x.name.as_str()))
            .map(|x| x.name)
            .collect::<Vec<_>>();

        if exchanges_without_destination {
            let surviving_queues = all_queues
                .into_iter()
                .filter(|x| x.exclusive || !queues_to_delete.iter().any(|dq| dq.name == x.name))
                .map(|x| x.name)
                .collect::<Vec<QueueName>>();
            filter_exchanges_without_destination(rc, vhost, exchanges, surviving_queues)?
        } else {
            exchanges
        }
    } else {
        vec![]
    };

    Ok(CollectedObjects {
        queues: queues_to_delete,
        exchanges: delete_exchanges,
    })
}

fn filter_exchanges_without_destination(
    rc: &RmqClient,
    vhost: &str,
    all_exchanges: Vec<ExchangeName>,
    queues: Vec<QueueName>,
) -> Result<Vec<ExchangeName>, Box<dyn std::error::Error>> {
    let bindings: HashMap<(String, BindingDestinationType), Vec<String>> = rc
        .list_bindings_in(vhost)?
        .into_iter()
        .fold(HashMap::new(), |mut acc, binding| {
            acc.entry((binding.destination, binding.destination_type))
                .and_modify(|vec| vec.push(binding.source.clone()))
                .or_insert(vec![binding.source]);
            acc
        });

    let mut survived_exchanges: HashSet<String> = HashSet::new();

    // All exchanges connected to queues are survived
    for queue in queues {
        if let Some(source_exchanges) = bindings.get(&(queue, BindingDestinationType::Queue)) {
            survived_exchanges.extend(source_exchanges.clone());
        }
    }

    // All other exchanges connected to survived exchanges are also survived
    let mut last_survived = survived_exchanges.clone();
    loop {
        let mut more_survived_exchanges = HashSet::new();
        for survived in last_survived {
            if let Some(source_exchanges) =
                bindings.get(&(survived, BindingDestinationType::Exchange))
            {
                more_survived_exchanges.extend(source_exchanges.clone());
            }
        }
        if more_survived_exchanges.is_empty(){
            break;
        }

        survived_exchanges.extend(more_survived_exchanges.clone());
        last_survived = more_survived_exchanges;
    }

    let all_exchanges: HashSet<_> = all_exchanges.into_iter().collect();
    Ok(all_exchanges.difference(&survived_exchanges).into_iter().cloned().collect())
}
