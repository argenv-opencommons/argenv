# Changelog

All notable changes to argenv are documented here.
## [0.3.2]


### Bug Fixes
- **resolve:** Bash completes a flag's values only where the flag applies ([#34](https://github.com/argenv-opencommons/argenv/pull/34)) ([de17bad])



## [0.3.1]


### Features
- **resolve:** Derive shell completions and a man page from a contract ([#32](https://github.com/argenv-opencommons/argenv/pull/32)) ([b5715fd])



## [0.3.0]


### Bug Fixes
- **lint:** A required input is only required when its own subcommand runs ([#22](https://github.com/argenv-opencommons/argenv/pull/22)) ([c2002ac])


### Documentation
- **examples:** The Rust Book's minigrep, with its arguments and environment declared once ([#27](https://github.com/argenv-opencommons/argenv/pull/27)) ([e15c2d3])


### Features
- **lint:** Misspelled flags and variables are errors by default, with a per-project LintPolicy ([#30](https://github.com/argenv-opencommons/argenv/pull/30)) ([e7e0236])
- **lint:** A flag given to a subcommand it does not belong to is an error; Finding is non_exhaustive ([#31](https://github.com/argenv-opencommons/argenv/pull/31)) **[BREAKING]** ([9ba7dc8])


### Miscellaneous
- Ship the license texts inside every published crate ([#23](https://github.com/argenv-opencommons/argenv/pull/23)) ([3673549])



## [0.2.0]


### Bug Fixes
- **docs:** Escape <target> in mock.rs doc comment ([c881a8e])
- **check:** A standalone --no-X flag is not automatically invalid; docs(examples): minimal + git ([a160b9d])


### Documentation
- **examples:** Proton + docker; fix(ci): examples were never actually tested ([2fd61f4])


### Refactoring
- Split into argenv (data) and argenv-resolve (parsing) **[BREAKING]** ([4ca6f84])


### Testing
- Real subprocess integration tests against a mock program ([1b9148f])


