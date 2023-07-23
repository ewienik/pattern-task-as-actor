# pattern-task-as-actor
Actor model using simple tokio tasks

# usage

```
cargo build
cargo r
```

## Design

There are three actors:
- Supervisor: stores and stops handles to all actors
- Ping: sends pong to Pong and stops Supervisor when after 10 balls
- Pong: sends ping to Ping

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
