mod args;
mod collector;

pub use crate::args::{Args, Action, DeleteOptions};
use crate::collector::{
    collect_objects, collect_queues, CollectedObjects, ExchangeName, Queue, RmqClient,
};
use rabbitmq_http_client::blocking_api::Client;
use url::Url;

pub fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
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
    let prefix = if dry_run { "[DRY RUN] " } else { "✓ " };
    println!("{}{}", prefix, message);
}
