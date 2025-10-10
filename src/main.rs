use clap::Parser;
use rabbitmq_http_client::blocking_api::Client;
use regex::Regex;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = ".+", help = "Regex filter for names")]
    filter: String,

    #[arg(short, long, default_value = "/", help = "Virtual host")]
    vhost: String,

    #[arg(short, long, default_value_t = false, help = "Delete exchanges")]
    exchanges: bool,

    #[arg(short, long, default_value_t = false, help = "Delete queues")]
    queues: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let endpoint = "http://localhost:15672/api";
    let username = "guest";
    let password = "guest";
    let rc = Client::new(endpoint, username, password);

    let re = Regex::new(&args.filter)?;

    let queues = rc.list_queues()?;
    for queue in queues {
        if args.vhost != queue.vhost || !re.is_match(&queue.name) {
            continue;
        }

        if args.queues {
            println!("Deleting queue {} - {}", queue.name, queue.message_count);
            rc.delete_queue(&queue.vhost, &queue.name, true)?;
        } else {
            println!("Purging {} - {}", queue.name, queue.message_count);
            rc.purge_queue(&queue.vhost, &queue.name)?;
        }
    }

    if args.exchanges {
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
            if args.vhost != exchange.vhost
                || !re.is_match(&exchange.name)
                || skip_exchanges.contains(&exchange.name.as_str())
            {
                continue;
            }

            println!("Deleting exchange {}", exchange.name);
            rc.delete_exchange(&exchange.vhost, &exchange.name, true)?;
        }
    }

    Ok(())
}
