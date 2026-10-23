//! A man page, in roff.

use super::{commands, is_plain, model, one_line, GenerateError, Model, Program};
use crate::Record;

/// Text for a roff line: backslashes and hyphens are escaped, and a line may not begin with a
/// control character (`.` or `'`).
fn text(input: &str) -> String {
    let flat = one_line(input).replace('\\', "\\e").replace('-', "\\-");
    if flat.starts_with('.') || flat.starts_with('\'') {
        format!("\\&{flat}")
    } else {
        flat
    }
}

/// Bold, for a flag or command name.
fn bold(word: &str) -> String {
    format!("\\fB{}\\fR", text(word))
}

fn italic(word: &str) -> String {
    format!("\\fI{}\\fR", text(word))
}

/// Generate the manual page (section 1).
///
/// Sections: NAME, SYNOPSIS (one line per subcommand), DESCRIPTION, COMMANDS, OPTIONS (grouped as
/// the contract groups them), ARGUMENTS, ENVIRONMENT, EXIT STATUS and SEE ALSO. The last two come
/// from [`Program`]; the date is a parameter so the output is reproducible.
pub fn man_page(program: &Program<'_>, records: &[Record]) -> Result<String, GenerateError> {
    let commands = commands(program)?;
    // An empty date is allowed (the footer is then left blank); anything else must be a token.
    if !program.date.is_empty() && !is_plain(program.date) {
        return Err(GenerateError::InvalidName(program.date.to_string()));
    }
    let Model { flags, positionals } = model(records);
    let name = program.name;

    let mut out = String::new();
    out.push_str(&format!(
        ".TH {} 1 \"{}\" \"{} {}\" \"User Commands\"\n",
        text(&name.to_uppercase()),
        // Validated as a plain token above, so it needs no escaping, and reads as a date.
        program.date,
        text(name),
        text(program.version)
    ));
    out.push_str(&format!(
        ".SH NAME\n{} \\- {}\n",
        text(name),
        text(program.about)
    ));

    // SYNOPSIS
    out.push_str(&format!(
        ".SH SYNOPSIS\n{} [{}]",
        bold(name),
        italic("OPTIONS")
    ));
    if commands.is_empty() {
        out.push('\n');
    } else {
        out.push_str(&format!(" {} [{}]\n", italic("COMMAND"), italic("ARGS")));
        for command in &commands {
            out.push_str(&format!(
                ".br\n{} {} [{}]",
                bold(name),
                bold(command.name),
                italic("OPTIONS")
            ));
            for positional in positionals.iter().filter(|p| p.available_for(command.name)) {
                let arg = italic(&positional.name);
                if positional.required {
                    out.push_str(&format!(" {arg}"));
                } else {
                    out.push_str(&format!(" [{arg}]"));
                }
            }
            out.push('\n');
        }
    }

    if !commands.is_empty() {
        out.push_str(".SH COMMANDS\n");
        for command in &commands {
            out.push_str(&format!(
                ".TP\n{}\n{}\n",
                bold(command.name),
                text(command.summary)
            ));
        }
    }

    // OPTIONS, grouped in the order the contract first mentions each group.
    let mut groups: Vec<&str> = Vec::new();
    for flag in &flags {
        if !groups.contains(&flag.group.as_str()) {
            groups.push(&flag.group);
        }
    }
    if !flags.is_empty() {
        out.push_str(".SH OPTIONS\n");
        for group in &groups {
            out.push_str(&format!(".SS {}\n", text(&group.to_uppercase())));
            for flag in flags.iter().filter(|f| f.group == *group) {
                let mut head = flag
                    .spellings()
                    .iter()
                    .filter(|s| !s.starts_with("--no-"))
                    .map(|s| bold(s))
                    .collect::<Vec<_>>()
                    .join(", ");
                if flag.takes_value {
                    head.push_str(&format!(" {}", italic(&flag.value_name)));
                }
                out.push_str(&format!(".TP\n{head}\n"));
                if !flag.summary.is_empty() {
                    out.push_str(&format!("{}\n", text(&flag.summary)));
                }
                let mut notes = Vec::new();
                if !flag.values.is_empty() {
                    notes.push(format!(
                        "One of: {}.",
                        flag.values
                            .iter()
                            .map(|v| bold(v))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                if let Some(default) = &flag.default {
                    notes.push(format!("Default: {}.", bold(default)));
                }
                if !flag.scope.is_empty() {
                    notes.push(format!(
                        "Applies to: {}.",
                        flag.scope
                            .iter()
                            .map(|s| bold(s))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                if !flag.env.is_empty() {
                    notes.push(format!(
                        "Environment: {}.",
                        flag.env
                            .iter()
                            .map(|e| bold(e))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                if !notes.is_empty() {
                    out.push_str(&format!(".br\n{}\n", notes.join(" ")));
                }
            }
        }
    }

    if !positionals.is_empty() {
        out.push_str(".SH ARGUMENTS\n");
        for positional in &positionals {
            out.push_str(&format!(".TP\n{}\n", italic(&positional.name)));
            if !positional.summary.is_empty() {
                out.push_str(&format!("{}\n", text(&positional.summary)));
            }
            if !positional.scope.is_empty() {
                out.push_str(&format!(
                    ".br\nApplies to: {}.\n",
                    positional
                        .scope
                        .iter()
                        .map(|s| bold(s))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
    }

    // ENVIRONMENT: every variable, with the flag it is the other door to.
    let with_env: Vec<_> = flags.iter().filter(|f| !f.env.is_empty()).collect();
    if !with_env.is_empty() {
        out.push_str(".SH ENVIRONMENT\n");
        for flag in with_env {
            let twin = flag
                .spellings()
                .iter()
                .find(|s| s.starts_with("--") && !s.starts_with("--no-"))
                .cloned();
            for variable in &flag.env {
                out.push_str(&format!(".TP\n{}\n", bold(variable)));
                match &twin {
                    Some(twin) => out.push_str(&format!(
                        "The same setting as {}; the flag wins when both are given.\n",
                        bold(twin)
                    )),
                    None => out.push_str(&format!("{}\n", text(&flag.summary))),
                }
            }
        }
    }

    if !program.exit_status.is_empty() {
        out.push_str(".SH EXIT STATUS\n");
        for (status, meaning) in program.exit_status {
            out.push_str(&format!(".TP\n{}\n{}\n", bold(status), text(meaning)));
        }
    }

    if !program.see_also.is_empty() {
        out.push_str(".SH SEE ALSO\n");
        out.push_str(
            &program
                .see_also
                .iter()
                .map(|page| text(page))
                .collect::<Vec<_>>()
                .join(", "),
        );
        out.push('\n');
    }
    Ok(out)
}
