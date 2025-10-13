mod collector;

use clap::Parser;
use rabbitmq_http_client::blocking_api::Client;
use url::Url;
use crate::collector::collect_objects;

const DRY_RUN_PREFIX: &str = "[DRY RUN] ";

#[derive(Parser, Debug)]
#[command(version, about = "Purges queues be default. Adding flag -q deletes them instead", long_about = None)]
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

    #[arg(short, long, default_value_t = false, help = "Dry run (change nothing)")]
    dry_run: bool,
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
        endpoint.as_str(),
        url.username(),
        url.password().expect("Password is missing"),
    );

    let objects= collect_objects(&rc, &args.vhost, &args.filter, args.queues, args.exchanges)?;

    for queue in objects.purge_queues {
        if args.dry_run {
            println!("{} Purging {} - {}", DRY_RUN_PREFIX, queue.name, queue.messages);
        }
        else{
            println!("✓ Purging {} - {}", queue.name, queue.messages);
            rc.purge_queue(&args.vhost, &queue.name)?;
        }
    }

    for queue in objects.delete_queues {
        if args.dry_run {
            println!("{} Deleting queue {} - {}", DRY_RUN_PREFIX, queue.name, queue.messages);
        }
        else{
            println!("✓ Deleting queue {} - {}", queue.name, queue.messages);
            rc.delete_queue(&args.vhost, &queue.name, true)?;
        }
    }
    
    for exchange in objects.delete_exchanges {
        if args.dry_run {
            println!("{} Deleting exchange {}", DRY_RUN_PREFIX, exchange);
        }
        else {
            println!("✓ Deleting exchange {}", exchange);
            rc.delete_exchange(&args.vhost, &exchange, true)?;
        }
    }

    Ok(())
}
