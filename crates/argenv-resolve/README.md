# argenv-resolve

Resolve an [argenv](https://docs.rs/argenv) contract against a real invocation
— argv and envp — with precedence, defaults, and linting.

`argenv` declares what a program accepts, as data, and does not read argv or
envp at all. This crate is the other half. See the crate-level docs for the
full picture.

```rust
use argenv_resolve::*;

contract! {
    HOST: String = Input {
        key: "host",
        ty:  Type::String,
        env: Some(Env::new("APP_HOST")),
        arg: Some(Arg { value_name: "HOST", ..Arg::long("host") }),
        ..Input::EMPTY
    };
}

fn main() {
    let resolved = Contract::parse_and_lint(OnProblems::FailOnError);
    let host = Contract::HOST.get_from(&resolved);
}
```
