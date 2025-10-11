use clap::Parser;
use rabbitmq_http_client::blocking_api::Client;
use regex::Regex;
use url::Url;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(
        short,
        long,
        default_value = "http://guest:guest@localhost:15672/api",
        help = "URL to RabbitMQ API"
    )]
    url: String,

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

    let url = Url::parse(&args.url)?;
    let endpoint = format!(
        "{}://{}:{}{}",
        url.scheme(),
        url.domain().expect("Domain is missing"),
        url.port().unwrap_or(443),
        url.path()
    );

    println!("Connecting to endpoint '{}' and vhost '{}'", endpoint, args.vhost);
    let rc = Client::new(
        &endpoint,
        url.username(),
        url.password().expect("Password is missing"),
    );

    let re = Regex::new(&args.filter)?;

    for queue in rc.list_queues()? {
        if args.vhost != queue.vhost || !re.is_match(&queue.name) {
            continue;
        }

        if queue.exclusive {
            println!("🚫  Skipping exclusive queue {}", queue.name);
            continue;
        }

        if args.queues {
            println!("✓ Deleting queue {} - {}", queue.name, queue.message_count);
            rc.delete_queue(&queue.vhost, &queue.name, true)?;
        } else if queue.message_count > 0 {
            println!("✓ Purging {} - {}", queue.name, queue.message_count);
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

            println!("✓ Deleting exchange {}", exchange.name);
            rc.delete_exchange(&exchange.vhost, &exchange.name, true)?;
        }
    }

    Ok(())
}
