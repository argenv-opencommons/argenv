# Terminology

A reference for the terms this project uses precisely and consistently.
Many of these words are used loosely or interchangeably in everyday tooling
discussion. That ambiguity is the source of most of the confusion about how
CLIs work. This document picks one meaning for each word and holds to it.

---

## Invocation

The complete call to a program: the program name followed by every token
the shell passes to it.

```
git  --no-pager  commit  --amend  -m  "fix typo"
```

The program name (`git`) is the first element of the argument vector but
is not part of what the program parses — it is the name the OS used to
find the binary. Everything after it is the **invocation surface**: the
part the program actually reads.

---

## Argument vector (argv)

The raw list of strings the operating system hands to the program when it
starts. The first element (`argv[0]`) is the program name. Everything from
`argv[1]` onward is the invocation.

```
["git", "--no-pager", "commit", "--amend", "-m", "fix typo"]
  argv[0]  argv[1]    argv[2]   argv[3]   argv[4]  argv[5]
```

The argument vector is the only channel argenv's **arg binding** reads from.
The **env binding** reads from the process environment, which is separate.

---

## Flag

A named token in the argument vector, identified by a `-` or `--` prefix
rather than by position. Flags come in two forms:

- **Long flag**: two dashes, kebab-case name: `--amend`, `--no-pager`, `--log-level`
- **Short flag**: one dash, single character: `-m`, `-v`, `-C`

Flags come in two behaviours:

### Switch

A flag that takes **no value**. Its presence in the invocation means `true`;
its absence means `false` (or whatever the declared default is). The parser
consumes exactly one token.

```
git commit --amend          # --amend is a switch; present → true
git commit                  # --amend absent → false (default)
git --no-pager log          # --no-pager is also a switch
```

A switch may be **negatable**: declaring `negatable: true` on a boolean input
produces a `--no-<name>` form that forces the value to `false` even when the
environment or a config file would set it to `true`.

```
program --hdr               # enable HDR
program --no-hdr            # explicitly disable it
```

### Option

A flag that takes **exactly one value**. The parser consumes two tokens (the
flag and the value) or one token in the `--flag=value` form.

```
git commit -m "fix typo"          # -m is an option; value is "fix typo"
git commit --message "fix typo"   # long form; same input, same value
git commit --message="fix typo"   # = form; same again
docker run --name mycontainer     # --name is an option
```

The string that follows an option is called its **value** (see below).

> **Why not "argument" for flag?**
> "Argument" is the traditional Unix word for any argv token, flags included.
> It is too broad to be useful here: `--amend`, `"fix typo"`, and `commit`
> are all "arguments" in that sense, but they behave completely differently.
> This project reserves "argument" for argenv's own `Arg` binding concept
> (see [Input](#input) below) and uses **flag** for the named `-`/`--` tokens.

---

## Positional

A token in the argument vector that is **not** a flag and **not** the value
of a preceding option. Positionals are identified by their position in the
sequence of non-flag tokens, not by a name.

```
cp   source.txt   dest.txt
     ^^^^^^^^^^^  ^^^^^^^^
     positional   positional
     [0]          [1]
```

```
git  --no-pager  commit  --amend  -m  "fix typo"
                 ^^^^^^
                 positional [0]
                 (--amend and "fix typo" are not positionals:
                  --amend is a switch; "fix typo" is the value of -m)
```

The parser collects positionals in order. They are available as
`resolution.positionals()` after the invocation is resolved.

The sequence `--` terminates flag parsing: every token after it is
treated as a positional, even if it starts with `-`.

```
git diff -- --cached          # "--cached" is a positional, not a flag
```

---

## Command (verb)

The **first positional** in an invocation, when it is used to select the
program's operating mode rather than to name a file or value. Also called
the **subcommand** when the program has a hierarchy.

```
git    commit  --amend
cargo  build   --release
apt    install nginx
docker run     --rm nginx
```

The command is always a positional — it has no `-` prefix. The program reads
`positionals()[0]` and dispatches on its value. Everything that follows is
interpreted in the context of that command.

A program that has no command is not wrong — many programs accept only flags
and positional values directly:

```
cp  --recursive  src/  dst/    # no command; cp takes only positionals and flags
grep  --count  pattern  file   # no command
```

---

## Value

The string that follows an **option** flag and supplies the input's content.
A value is not a flag and not a positional — it is consumed by the preceding
option and is not independently accessible.

```
git commit -m     "fix typo"       # "fix typo" is the value of -m
docker run --name mycontainer      # "mycontainer" is the value of --name
curl --output -                    # "-" is the value of --output (not a flag)
```

Values that contain spaces must be quoted in the shell. The program sees the
unquoted string.

---

## Token

One element within a **list** value. A list input accepts multiple items
in a single value, separated by declared separator characters.

```
program --hud fps,gpuload,memory
                ^^^  ^^^^^^^^^  ^^^^^^^
                tok  token      token (separator: ,)

program --hud "fps;gpuload"
                ^^^  ^^^^^^^
                tok  token (separator: ;)
```

Tokens are also what `Type::Enum` checks values against: the value must
exactly match one declared token.

---

## Environment variable (env var)

A key–value pair from the **process environment**, not from the argument
vector. Env vars arrive through a completely separate channel and are set
before the program starts.

```
GIT_AUTHOR_NAME="Alice"  git commit -m "msg"
^^^^^^^^^^^^^^^^^^^^^^^^
env var, not a flag; consumed by the env binding of the author_name input

MYAPP_LOG_LEVEL=warn  program --hud fps
^^^^^^^^^^^^^^^^^^^^^^^^
sets the log_level input via its env binding;
--hud is unrelated and arrives via its arg binding
```

argenv models the env channel with an **Env binding** and the argv channel
with an **Arg binding**. Both bindings address the same underlying **input**.

---

## Input

argenv's central concept. One setting a program accepts — its **identity**,
its **domain**, and the **doors** it can arrive through.

```rust
pub const LOG_LEVEL: Input<LogLevel> = Input {
    key:     "log_level",           // identity: transport-free, snake_case
    ty:      Type::Enum,            // domain: what kind of value
    allowed: LogLevel::TOKENS,      // domain: which values are legal
    env:     Some(Env::new("APP_LOG_LEVEL")),   // env door
    arg:     Some(Arg { value_name: "LEVEL",    // argv door
                        ..Arg::pair("log-level", 'l') }),
    ..Input::EMPTY
};
```

`--log-level warn` and `APP_LOG_LEVEL=warn` are **not** two settings.
They are one setting (`log_level`) with two doors. Both doors share the
same type, the same domain, and the same default.

The `key` is the identity. It is transport-free: it is not a flag name and
not a variable name. It reads the same in every language a binding is
generated for.

---

## Binding

How an input is **addressed** — how it is named in a specific transport
channel.

| Binding | Transport | Example |
|---------|-----------|---------|
| **Arg binding** | argument vector | `--log-level warn`, `-l warn` |
| **Env binding** | process environment | `APP_LOG_LEVEL=warn` |

An input may have one binding, both, or (for future expansion) more.
An input with no binding at all is rejected at declaration time — nothing
could ever supply it a value.

The binding is **not** the input. The input is the setting; the binding is
the address. Renaming `APP_LOG_LEVEL` to `MYAPP_LOG_LEVEL` is a binding
change; the input (`log_level`) stays the same.

---

## Resolution

The result of matching a specific invocation against the declared model.
Each resolved input carries:

- the **raw value** (the string before parsing into the typed `T`)
- the **source** — which door it arrived through: `Arg`, `Env`, or `Default`

`resolution.positionals()` holds the non-flag tokens that were not consumed
by any option, in order.

---

## Quick reference

| Term | Meaning in this project |
|------|------------------------|
| **invocation** | Everything after the program name |
| **argv** | The raw OS list; `argv[0]` is the program name |
| **flag** | A named argv token starting with `-` or `--` |
| **switch** | A flag with no value; presence means `true` |
| **option** | A flag that consumes one following value |
| **positional** | A non-flag, non-consumed token, identified by position |
| **command / verb** | `positionals()[0]`, used for dispatch |
| **value** | The string an option consumes |
| **token** | One item within a list or enum value |
| **env var** | A key–value pair from the process environment |
| **input** | One declared setting: identity + domain + bindings |
| **binding** | How an input is addressed in one transport (Arg or Env) |
| **resolution** | The matched values after parsing an invocation |

---

## What "argument" means here

The word **argument** is deliberately avoided in prose because it is
overloaded beyond usefulness:

- In POSIX tradition: any token passed to the program (flags included)
- In everyday speech: sometimes means "flag", sometimes "positional"
- In `argc`/`argv`: the whole list, flags included
- In argenv's code: `Arg` is the name of the argv binding struct

When you see **Arg** in argenv's source, it means specifically the argv
binding of an input — the struct that carries `long`, `short`, and related
fields. It does not mean "positional" and it does not mean "any argv token."
This document uses **flag** and **positional** for the two kinds of argv
token, and reserves **Arg** / **arg binding** for the argenv concept.
