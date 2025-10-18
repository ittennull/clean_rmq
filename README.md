Small tool to quickly purge all messages in RabbitMQ. It can also delete the queues and exchanges if corresponding options are passed to it.

See output of `clean_rmq help` for more details and examples:
```bash
Cleans RabbitMQ by purging queues or deleting queues and exchanges.
See examples below. Get help on each command like this clean_rmq help delete

Usage: clean_rmq [OPTIONS] [COMMAND]

Commands:
  purge   Purge queues matching filter. This is the default command if nothing is specified
  delete  Delete queues or exchanges or both
  help    Print this message or the help of the given subcommand(s)

Options:
  -u, --url <URL>
          URL to RabbitMQ API
          
          [default: http://guest:guest@localhost:15672/api]

  -v, --vhost <VHOST>
          Virtual host
          
          [default: /]

  -d, --dry-run
          Dry run (change nothing)

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version


Examples:
- Purge all queues on localhost RabbitMQ server
  clean_rmq
  or
  clean_rmq purge

- Any operation in dry run mode changes nothing, it only prints out the intendied actions. Short flag is '-d' or long flag is '--dry-run'
  clean_rmq --dry-run purge

- Purge queues with names ending with "_error"
  clean_rmq purge -f '.*_error'

- Connect to a remote RabbitMQ server and vhost 'myapp' and purge queues with names starting with "temp_"
  clean_rmq -u 'http://user:password@remotehost:15672/api' --vhost myapp purge -f '^temp_.*'

- Delete all queues and exchanges to get a clean state as if you have just started RabbitMQ up. Exclusive queues are never deleted.
  clean_rmq delete -q -e
  or you can combine short flags
  clean_rmq delete -qe

- Delete only queues without consumers that match the name filter 'process-.*'
  clean_rmq delete -q --queues-without-consumers -f 'process-.*'
  
- Delete all exchanges that are not directly on indirectly bound to any queue. A message published to such an exchange would be lost.
  clean_rmq delete -e --exchanges-without-destination
  
- Same as above. Also delete queues without consumers that match the name filter 'process-.*'.
  If an exchange is bound directly or indirectly to a non-exclusive queue matching the filter, e.g. 'process-123', it will be deleted too because this operation deletes this queue and the exchange becomes unbound
  clean_rmq delete -e --exchanges-without-destination -q --queues-without-consumers -f 'process-.*'
```

And here is output of `clean_rmq help delete` for more details on the delete command:
```bash
Delete queues or exchanges or both

Usage: clean_rmq delete [OPTIONS]

Options:
  -q, --queues                         Delete queues
      --queues-without-consumers       Delete queues only if they don't have consumers. Works only if -q|--queues is also specified
  -f, --queue-filter <QUEUE_FILTER>    Regex filter for queue names. Skip queues that don't match this filter. Works only if -q|--queues is also specified [default: .+]
  -e, --exchanges                      Delete exchanges
      --exchanges-without-destination  Delete exchanges without destination or if all of the destination's exchanges don't end up in a queue. If an exchange is bound to a queue that is also deleted in this operation (using flag -q|--queues), this exchange will be deleted too unless it's also bound to any queue that survives
  -h, --help                           Print help
  -V, --version                        Print version
```

## Motivation
There are 2 main use cases for this tool:
1. During development and testing it is often necessary to quickly clear out all messages in RabbitMQ to get a clean state. This tool makes it easy to do that from the command line. With a clean state it's easier to spot errors
2. In production, sometimes it is necessary to delete obsolete queues or exchanges. As your system changes, so does the queue and exchange topology. Old exchanges often stick around, they clutter RabbitMQ and in case of queues, they can consume storage space.