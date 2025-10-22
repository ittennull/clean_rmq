use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about = r#"Cleans RabbitMQ by purging queues or deleting queues and exchanges.
See examples below"#,
    long_about = None, after_long_help = color_print::cstr!(r#"
<bold>Examples</>:
- Purge all queues on localhost RabbitMQ server
  <green><i>clean_rmq</></>
  or
  <green><i>clean_rmq purge</></>

- Any operation in dry run mode changes nothing, it only prints out the intendied actions. Short flag is '-d' or long flag is '--dry-run'
  <green><i>clean_rmq --dry-run purge</></>

- Purge queues with names ending with "_error"
  <green><i>clean_rmq purge -f '.*_error'</></>
  
- Purge queues with names ending with "_error" but not starting with "critical_" or "important_"
  <green><i>clean_rmq purge -f '.*_error' --exclude-queue-filter 'critical_.*' --exclude-queue-filter 'important_.*'</></>

- Connect to a remote RabbitMQ server and vhost 'myapp' and purge queues with names starting with "temp_"
  <green><i>clean_rmq -u 'http://user:password@remotehost:15672/api' --vhost myapp purge -f '^temp_.*'</></>

- Delete all queues and exchanges to get a clean state as if you have just started RabbitMQ up. Exclusive queues are never deleted.
  <green><i>clean_rmq delete -q -e</></>
  or you can combine short flags
  <green><i>clean_rmq delete -qe</></>

- Delete only queues without consumers that match the name filter 'process-.*'
  <green><i>clean_rmq delete -q --queues-without-consumers -f 'process-.*'</></>

- Delete all exchanges that are not directly on indirectly bound to any queue. A message published to such an exchange would be lost.
  <green><i>clean_rmq delete -e --exchanges-without-destination</></>

- Same as above. Also delete queues without consumers that match the name filter 'process-.*'.
  If an exchange is bound directly or indirectly to a non-exclusive queue matching the filter, e.g. 'process-123', it will be deleted too because this operation deletes this queue and the exchange becomes unbound
  <green><i>clean_rmq delete -e --exchanges-without-destination -q --queues-without-consumers -f 'process-.*'</></>
"#))]
pub struct Args {
    #[arg(
        short,
        long,
        default_value = "http://guest:guest@localhost:15672/api",
        help = "URL to RabbitMQ API"
    )]
    pub url: String,

    #[arg(short, long, default_value = "/", help = "Virtual host")]
    pub vhost: String,

    #[arg(
        short,
        long,
        default_value_t = false,
        help = "Dry run (change nothing)"
    )]
    pub dry_run: bool,

    #[command(subcommand)]
    pub action: Option<Action>,
}

#[derive(Subcommand)]
pub enum Action {
    #[command(version, about = "Purge queues matching filter. This is the default command if nothing is specified. The command first collects all the queues that match the filter and then removes the queues that match any of the exclude filters", long_about = None)]
    Purge {
        #[arg(short = 'f', long, default_value = ".+", help = "Regex filter for names")]
        queue_filter: String,

        #[arg(long, help = "Regex filter that matches queue names to be excluded from purging. The flag can be specified multiple times")]
        exclude_queue_filter: Vec<String>,
    },

    #[command(version, about = "Delete queues or exchanges or both", long_about = None)]
    Delete(DeleteOptions),
}

#[derive(clap::Args)]
pub struct DeleteOptions {
    #[arg(short, long, default_value_t = false, help = "Delete queues")]
    pub queues: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "Delete queues only if they don't have consumers. Works only if -q|--queues is also specified"
    )]
    pub queues_without_consumers: bool,

    #[arg(
        short = 'f',
        long,
        default_value = ".+",
        help = "Regex filter for queue names. Skip queues that don't match this filter. Works only if -q|--queues is also specified"
    )]
    pub queue_filter: String,

    #[arg(
        long,
        help = "Regex filter that matches queue names to be excluded from deletion. The flag can be specified multiple times"
    )]
    pub exclude_queue_filter: Vec<String>,

    #[arg(short, long, default_value_t = false, help = "Delete exchanges")]
    pub exchanges: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "Delete exchanges without destination or if all of the destination's exchanges don't end up in a queue. If an exchange is bound to a queue that is also deleted in this operation (using flag -q|--queues), this exchange will be deleted too unless it's also bound to any queue that survives"
    )]
    pub exchanges_without_destination: bool,
}
