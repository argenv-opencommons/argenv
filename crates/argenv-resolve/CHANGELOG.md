# Changelog

All notable changes to argenv are documented here.
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


