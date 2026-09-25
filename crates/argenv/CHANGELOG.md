# Changelog

All notable changes to argenv are documented here.
## [0.2.0]


### Bug Fixes
- **check:** A standalone --no-X flag is not automatically invalid; docs(examples): minimal + git ([a160b9d])


### CI/CD
- **deps:** none — Dependabot and a README badge, no crate behavior change. ([88570e1])


### Documentation
- **example:** Apt — gated positionals, two branches sharing a slot ([04f4f3a])
- **example:** Subcommand_minimal - the floor for subcommands ([0e308aa])


### Features
- Positional arguments and gated inputs ([64225db])
- Argenv::parse() and lint(&Resolution) — remove the boilerplate ([216c652])
- Model! macro, OnProblems, handle_problems, parse_and_lint ([36eab48])
- Input<T, C> - subcommands as a typed list, not a string ([c8479d7])
- **contract!:** Accept an optional subcommand-enum type; docs(example): subcommand_rich ([a000fd1])


### Refactoring
- Remove positional args and gated inputs, rename model! to contract! **[BREAKING]** ([01cd0e4])
- Split into argenv (data) and argenv-resolve (parsing) **[BREAKING]** ([4ca6f84])


### Styling
- Apply rustfmt (long assertion lines in positional.rs) ([4cab965])
- Apply rustfmt ([17c3008])
- Apply rustfmt ([cbbf472])
- Apply rustfmt to apt.rs ([e45d160])


