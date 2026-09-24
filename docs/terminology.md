# CLI Terminology

Terms as the mainstream community and standards have settled on them. Sources:
POSIX.1-2017 §12 (the formal standard), GNU Coding Standards, clap (the
dominant Rust CLI library), and widespread community convention.

---

## The actors in an invocation

```
git   --no-pager   commit   --amend   -m   "fix typo"
^^^   ^^^^^^^^^^   ^^^^^^   ^^^^^^^^  ^^   ^^^^^^^^^^
 |       |           |         |       |       |
 |    option       subcommand  |   option   option-argument
program (flag)              option   (short)
                            (flag)
```

---

## Program

The binary. `argv[0]`. Not part of the surface the program parses —
it is the name the OS used to find the executable.

```
git commit -m "msg"
^^^
program
```

---

## Option

A named argument, identified by a `-` or `--` prefix. **POSIX calls these
"options" (historically also "flags").** Two forms:

- **Short option**: a single dash and one character: `-m`, `-v`, `-C`
- **Long option**: two dashes and a word: `--amend`, `--no-pager`, `--message`

Options come in two behaviours:

### Flag (boolean option)

An option that takes **no value**. Presence means `true`, absence means `false`.
Clap's documentation calls these "Flags / Switches".

```
git commit --amend          # flag; --amend present → true
git push --force            # flag
grep --count pattern file   # --count is a flag
```

### Option with argument

An option that takes **exactly one** following value. POSIX calls that value
the **option-argument**.

```
git commit -m "fix typo"    # -m takes an option-argument: "fix typo"
git clone --depth 1 <url>   # --depth takes an option-argument: 1
curl --output -             # --output takes an option-argument: -
docker run --name web nginx # --name takes an option-argument: web
```

Option-argument forms accepted in practice:

```
--level warn          # space-separated (most common)
--level=warn          # = form, no space
-l warn               # short, space-separated
-lwarn                # short, no space
```

---

## Positional argument

An argument identified by **position** rather than by name. Not prefixed
with `-`. Also called **operand** in POSIX.

```
cp   source.txt   dest.txt
     ^^^^^^^^^^^  ^^^^^^^^
     positional   positional
     [0]          [1]

git commit -m "msg"
# no positional arguments here; "msg" is the option-argument of -m
```

POSIX Guideline 9 says options should precede operands. In practice,
many tools (including git) accept them in any order by default.

The sequence `--` ends option parsing. Every token after it is treated as
a positional even if it starts with `-`:

```
git diff -- --cached          # "--cached" is a positional, not an option
rm -- -rf                     # "-rf" treated as a filename
```

---

## Subcommand

The first positional argument when it selects the program's operating mode.
Also called **command**, **verb**, or in POSIX-descended usage, an operand
used as a dispatch key. This is what clap calls a "subcommand."

```
git    commit   --amend
cargo  build    --release
apt    install  nginx
docker run      --rm nginx
       ^^^^^^^
       subcommand (first positional; selects the mode)
```

Every token that follows is interpreted in the context of that subcommand.
Options before the subcommand are **global options**; options after it are
**subcommand options** (or **local options**). Many tools enforce this
distinction:

```
git   --no-pager   commit   --amend
      ^^^^^^^^^^            ^^^^^^^
      global option         subcommand option (git commit's own)
```

Not every program has a subcommand. Programs with no dispatch just take
positional arguments directly:

```
cp  src/  dst/          # no subcommand; two positional arguments
grep  pattern  file     # no subcommand; pattern and file are positionals
```

---

## Environment variable

A key–value pair from the **process environment**, set before the program
starts, through a completely separate channel from argv.

```
GIT_AUTHOR_NAME="Alice"  git commit -m "msg"
APP_LOG_LEVEL=warn       program --hud fps
DOCKER_HOST=tcp://...    docker ps
```

Environment variables are the second standard way to configure a program.
Together with options, they cover the two surfaces the OS defines for
configuring a process at invocation time.

---

## Quick reference

| Term | What it is | Example |
|------|-----------|---------|
| **program** | `argv[0]`, the binary | `git` |
| **option** | Named `-`/`--` argument | `--amend`, `-m` |
| **flag** | Boolean option (no value) | `--verbose`, `--force` |
| **option-argument** | Value following an option | `"fix typo"` in `-m "fix typo"` |
| **positional argument** | Non-option token, by position | `source.txt` in `cp source.txt dst` |
| **subcommand** | First positional, used for dispatch | `commit` in `git commit` |
| **global option** | Option before the subcommand | `--no-pager` in `git --no-pager log` |
| **subcommand option** | Option scoped to one subcommand | `--amend` in `git commit --amend` |
| **environment variable** | Key=value from the process env | `GIT_AUTHOR_NAME=Alice` |

---

## What "argument" means in general

In common usage, **argument** means anything passed to a program — options
and positionals alike. That is why `argc`/`argv` includes both. It is too
broad to be precise in documentation, so this project prefers the specific
terms above.

When you see `Arg` in argenv's source code, it means specifically the
argv binding of an input (the struct describing which flag(s) an input
answers to). It is not a synonym for "positional argument."
