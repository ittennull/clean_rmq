mod collector;

use crate::collector::{
    collect_objects, collect_queues, CollectedObjects, ExchangeName, Queue, RmqClient,
};
use clap::{Parser, Subcommand};
use rabbitmq_http_client::blocking_api::Client;
use url::Url;

#[derive(Parser)]
#[command(version, about = "Cleans RabbitMQ by purging queues or deleting queues and exchanges", long_about = None)]
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

    #[command(subcommand)]
    action: Option<Action>,
}

#[derive(Subcommand)]
enum Action {
    #[command(version, about = "Purge queues matching filter. This is the default command if nothing is specified", long_about = None)]
    Purge {
        #[arg(short, long, default_value = ".+", help = "Regex filter for names")]
        filter: String,
    },

    #[command(version, about = "Delete queues or exchanges or both", long_about = None)]
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
        help = "Regex filter for queue names. Skip queues that don't match this filter. Works only if -q|--queues is also specified"
    )]
    queue_filter: String,

    #[arg(short, long, default_value_t = false, help = "Delete exchanges")]
    exchanges: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "Delete exchanges without destination or if all of the destination's exchanges don't end up in a queue. If an exchange is bound to a queue that is also deleted in this operation (using flag -q|--queues), this exchange will be deleted too unless it's also bound to any queue that survives"
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

        print_line(
            dry_run,
            &format!("Purging queue {} - {}", queue.name, queue.messages),
        );
        if !dry_run {
            rc.purge_queue(vhost, &queue.name)?;
        }
    }

    let num_exclusive = queues.iter().filter(|q| q.exclusive).count();
    println!(
        "Purged {} queues, skipped {} exclusive queues",
        queues.len() - num_exclusive,
        num_exclusive
    );

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

        print_line(
            dry_run,
            &format!("Deleting queue {} - {}", queue.name, queue.messages),
        );
        if !dry_run {
            rc.delete_queue(vhost, &queue.name, true)?;
        }
    }

    for exchange in exchanges {
        print_line(dry_run, &format!("Deleting exchange {}", exchange));
        if !dry_run {
            rc.delete_exchange(vhost, &exchange, true)?;
        }
    }

    let num_exclusive = queues.iter().filter(|q| q.exclusive).count();
    println!(
        "Deleted {} queues, {} exchanges, skipped {} exclusive queues",
        queues.len() - num_exclusive,
        exchanges.len(),
        num_exclusive
    );

    Ok(())
}

fn print_line(dry_run: bool, message: &str) {
    let prefix = if dry_run { "[DRY RUN]" } else { "✓ " };
    println!("{}{}", prefix, message);
}
