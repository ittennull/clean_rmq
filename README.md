# RabbitMQ cleaner

A tool to quickly purge all messages in RabbitMQ. It can also delete the queues and exchanges if corresponding options are passed to it.

See output of `clean_rmq help` for more details and examples:
```
Cleans RabbitMQ by purging queues or deleting queues and exchanges.
See examples below

Usage: clean_rmq [OPTIONS] [COMMAND]

Commands:
  purge   Purge queues matching filter. This is the default command if nothing is specified.
          The command first collects all the queues that match the filter and then excludes the queues that match any of the exclude filters
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

- Purge queues with names ending with "_error" but not starting with "critical_" or "important_"
  clean_rmq purge -f '.*_error' --exclude-queue-filter 'critical_.*' --exclude-queue-filter 'important_.*'

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
  If an exchange is bound directly or indirectly to a non-exclusive queue matching the filter, e.g. 'process-123',
  it will be deleted too because this operation deletes this queue and the exchange becomes unbound
  clean_rmq delete -e --exchanges-without-destination -q --queues-without-consumers -f 'process-.*'
```

## Purge queues
`clean_rmq help purge`:
```
Purge queues matching filter. This is the default command if nothing is specified.
The command first collects all the queues that match the filter and then excludes the queues that match any of the exclude filters

Usage: clean_rmq purge [OPTIONS]

Options:
  -f, --queue-filter <QUEUE_FILTER>
          Regex filter for names [default: .+]
      --exclude-queue-filter <EXCLUDE_QUEUE_FILTER>
          Regex filter that matches queue names to be excluded from purging. The flag can be specified multiple times
  -h, --help
          Print help
  -V, --version
          Print version
```

#### Delete queue and exchanges
`clean_rmq help delete`:
```
Delete queues or exchanges or both

Usage: clean_rmq delete [OPTIONS]

Options:
  -q, --queues
          Delete queues
      --queues-without-consumers
          Delete queues only if they don't have consumers. Works only if -q|--queues is also specified
  -f, --queue-filter <QUEUE_FILTER>
          Regex filter for queue names. Skip queues that don't match this filter. Works only if -q|--queues is also specified [default: .+]
      --exclude-queue-filter <EXCLUDE_QUEUE_FILTER>
          Regex filter that matches queue names to be excluded from deletion. The flag can be specified multiple times
  -e, --exchanges
          Delete exchanges
      --exchanges-without-destination
          Delete exchanges without destination or if all of the destination's exchanges don't end up in a queue.
          If an exchange is bound to a queue that is also deleted in this operation (using flag -q|--queues), this exchange will be deleted too unless it's also bound to any queue that survives
  -h, --help
          Print help
  -V, --version
          Print version
```

## Motivation
There are 2 main use cases for this tool:
1. During development and testing it is often necessary to quickly clear out all messages in RabbitMQ to get a clean state. This tool makes it easy to do that from the command line. With a clean state it's easier to spot errors
2. In production, sometimes it is necessary to delete obsolete queues or exchanges. As your system changes, so does the queue and exchange topology. Old exchanges often stick around, they clutter RabbitMQ and in case of queues, they can consume storage space.

## Installation
1. Download a binary for your operating system from the releases page
2. Rename it to `clean_rmq`. It's not necessary, but short name is easier to type
3. Make the binary executable (Linux, macOS): `chmod +x clean_rmq`
4. (Optional) Move the binary to a directory in your PATH
5. (Optional) On macOS you may need to run `xattr -dr com.apple.quarantine clean_rmq` to be able to run the binary