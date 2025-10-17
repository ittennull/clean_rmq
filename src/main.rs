mod collector;

use crate::collector::{
    CollectedObjects, ExchangeName, Queue, RmqClient, collect_objects, collect_queues,
};
use clap::{Parser, Subcommand};
use rabbitmq_http_client::blocking_api::Client;
use url::Url;

const DRY_RUN_PREFIX: &str = "[DRY RUN] ";

#[derive(Parser)]
#[command(version, about = "Purges queues or deletes queues and exchanges", long_about = None)]
struct Args {
    #[arg(
        short,
        long,
        default_value = "http://guest:guest@localhost:15672/api",
        help = "URL to RabbitMQ API"
    )]
    url: String,

    #[arg(short, long, default_value = "/", help = "Virtual host")]
    vhost: String,

    #[arg(
        short,
        long,
        default_value_t = false,
        help = "Dry run (change nothing)"
    )]
    dry_run: bool,

    #[command(subcommand, help = "Defaults to 'purge' command")]
    action: Option<Action>,
}

#[derive(Subcommand)]
enum Action {
    #[command(version, about = "Purge queues matching filter", long_about = None)]
    Purge {
        #[arg(short, long, default_value = ".+", help = "Regex filter for names")]
        filter: String,
    },

    #[command(version, about = "Delete queues or exchanges or both matching filter", long_about = None)]
    Delete(DeleteOptions),
}

#[derive(clap::Args)]
struct DeleteOptions {
    #[arg(short, long, default_value_t = false, help = "Delete queues")]
    queues: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "Delete queues only if they don't have consumers. Works only if -q|--queues is also specified"
    )]
    queues_without_consumers: bool,

    #[arg(
        short = 'f',
        long,
        default_value = ".+",
        help = "Regex filter for queue names"
    )]
    queue_filter: String,

    #[arg(short, long, default_value_t = false, help = "Delete exchanges")]
    exchanges: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "Delete exchanges without destination or if all of the destination's exchanges don't end up in a queue. If an exchange is bound to a queue that is also deleted in this operation (using flags -q|--queues), this exchange will be deleted too"
    )]
    exchanges_without_destination: bool,
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

    println!(
        "Connecting to endpoint '{}' and vhost '{}'",
        endpoint, args.vhost
    );
    let rc = Client::new(
        endpoint.as_str(),
        url.username(),
        url.password().expect("Password is missing"),
    );

    let action = args.action.unwrap_or_else(|| Action::Purge {
        filter: ".+".to_string(),
    });

    match action {
        Action::Purge { filter } => {
            let queues = collect_queues(&rc, &args.vhost, &filter)?;
            purge(&rc, &args.vhost, args.dry_run, &queues)?;
        }
        Action::Delete(options) => {
            let CollectedObjects { queues, exchanges } = collect_objects(
                &rc,
                &args.vhost,
                options.queues,
                options.queues_without_consumers,
                &options.queue_filter,
                options.exchanges,
                options.exchanges_without_destination,
            )?;
            delete(&rc, &args.vhost, args.dry_run, &queues, &exchanges)?;
        }
    }

    Ok(())
}

fn purge(
    rc: &RmqClient,
    vhost: &str,
    dry_run: bool,
    queues: &Vec<Queue>,
) -> Result<(), Box<dyn std::error::Error>> {
    for queue in queues {
        if queue.exclusive {
            println!("🚫  Skipping exclusive queue {}", queue.name);
            continue;
        }

        if dry_run {
            println!(
                "{} Purging {} - {}",
                DRY_RUN_PREFIX, queue.name, queue.messages
            );
        } else {
            println!("✓ Purging {} - {}", queue.name, queue.messages);
            rc.purge_queue(vhost, &queue.name)?;
        }
    }

    Ok(())
}

fn delete(
    rc: &RmqClient,
    vhost: &str,
    dry_run: bool,
    queues: &Vec<Queue>,
    exchanges: &Vec<ExchangeName>,
) -> Result<(), Box<dyn std::error::Error>> {
    for queue in queues {
        if queue.exclusive {
            println!("🚫  Skipping exclusive queue {}", queue.name);
            continue;
        }

        if dry_run {
            println!(
                "{} Deleting queue {} - {}",
                DRY_RUN_PREFIX, queue.name, queue.messages
            );
        } else {
            println!("✓ Deleting queue {} - {}", queue.name, queue.messages);
            rc.delete_queue(vhost, &queue.name, true)?;
        }
    }

    for exchange in exchanges {
        if dry_run {
            println!("{} Deleting exchange {}", DRY_RUN_PREFIX, exchange);
        } else {
            println!("✓ Deleting exchange {}", exchange);
            rc.delete_exchange(vhost, &exchange, true)?;
        }
    }

    Ok(())
}
