Small tool to quickly purge all messages in RabbitMQ. It can also delete the queues and exchanges if corresponding options are passed to it.

See `clean_rmq --help` for more details:
```bash
Usage: clean_rmq [OPTIONS]

Options:
  -u, --url <URL>        URL to RabbitMQ API [default: http://guest:guest@localhost:15672/api]
  -f, --filter <FILTER>  Regex filter for names [default: .+]
  -v, --vhost <VHOST>    Virtual host [default: /]
  -e, --exchanges        Delete exchanges
  -q, --queues           Delete queues
  -h, --help             Print help
  -V, --version          Print version
```