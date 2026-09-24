//! A fully-documented model using every argenv feature.
//!
//! This is what a real program looks like end to end: declare the model with
//! doc comments, call `parse_and_lint`, read typed values. Nothing else.
//!
//! ```text
//! cargo run -p argenv --example consumer -- --log-level warn -v
//! cargo run -p argenv --example consumer -- --bogus-flag
//! cargo test -p argenv --example consumer
//! ```
use argenv::*;
use std::path::PathBuf;

/// A render scale constrained to `0.1..=1.0`, stored as per-mille.
///
/// A bare `f32` would admit `5.0` and `NaN`, which are not render scales.
/// Storing the bounded decimal as an integer makes the range check
/// `const`-evaluable and sidesteps float equality entirely.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(into = "f32")]
pub struct RenderScale(u16);

impl RenderScale {
    pub const FULL: RenderScale = RenderScale::from_permille(1000);

    pub const fn from_permille(p: u16) -> RenderScale {
        assert!(
            p >= 100 && p <= 1000,
            "render scale must be within 0.1..=1.0"
        );
        RenderScale(p)
    }

    pub fn as_f32(self) -> f32 {
        self.0 as f32 / 1000.0
    }
}
impl From<RenderScale> for f32 {
    fn from(r: RenderScale) -> f32 {
        r.as_f32()
    }
}
impl FromRaw for RenderScale {
    fn from_raw(s: &str) -> Option<RenderScale> {
        let v: f32 = s.trim().parse().ok()?;
        if !(0.1..=1.0).contains(&v) {
            return None;
        }
        Some(RenderScale((v * 1000.0).round() as u16))
    }
}

// Compile-checked successor reference: rename or delete `HUD` and this stops
// compiling, so the pointer can never dangle.
fn hud_replacement() -> &'static str {
    Model::HUD.key
}

model! {
    /// Both doors: a flag and a variable, one setting.
    LOG_LEVEL: LogLevel = Input {
        key:       "log_level",
        ty:        Type::Enum,
        default:   Some(LogLevel::Info),
        allowed:   LogLevel::TOKENS,
        env:       Some(Env::new("MYAPP_LOG_LEVEL")),
        arg:       Some(Arg { value_name: "LEVEL", ..Arg::pair("log-level", 'l') }),
        stability: Stability::Stable,
        since:     Since::This,
        group:     "logging",
        example:   "--log-level warn",
        reviewed:  Some(ReviewDate::parse("2026-07-21")),
        summary:   "Log verbosity",
        ..Input::EMPTY
    };

    /// A boolean flag: presence is the value, and it can be negated.
    HDR: bool = Input {
        key:       "hdr",
        ty:        Type::Bool,
        default:   Some(false),
        env:       Some(Env::new("MYAPP_HDR")),
        arg:       Some(Arg { negatable: true, ..Arg::long("hdr") }),
        stability: Stability::Stable,
        since:     Since::This,
        group:     "display",
        maps_to:   Some(ConfigKeyRef::new("display.enableHDR")),
        example:   "--hdr",
        observe:   Some("hdr_enabled"),
        reviewed:  Some(ReviewDate::parse("2026-07-21")),
        summary:   "Expose HDR output to the application",
        ..Input::EMPTY
    };

    /// A repeatable list: `--hud fps --hud gpuload` accumulates.
    HUD: String = Input {
        key:        "hud",
        ty:         Type::List,
        allowed:    &["fps", "frametimes", "gpuload", "memory", "version"],
        separators: &[',', ';'],
        env:        Some(Env::new("MYAPP_HUD")),
        arg:        Some(Arg { repeatable: true, value_name: "ELEMENT", ..Arg::long("hud") }),
        stability:  Stability::Stable,
        since:      Since::This,
        group:      "display",
        example:    "--hud fps --hud gpuload",
        reviewed:   Some(ReviewDate::parse("2026-07-21")),
        summary:    "Overlay elements to display",
        ..Input::EMPTY
    };

    /// Environment only: no flag, because it is set by a launcher, not a person.
    LOG_PATH: PathBuf = Input {
        key:   "log_path",
        ty:    Type::Path,
        // The variable was renamed; the old name is still honoured.
        env:   Some(Env { name: "MYAPP_LOG_PATH", aliases: &["MYAPP_LOGFILE"] }),
        group: "logging",
        ..Input::EMPTY
    };

    /// Argument only: a bounded decimal, using a program-local domain type.
    RENDER_SCALE: RenderScale = Input {
        key:       "render_scale",
        ty:        Type::Float,
        default:   Some(RenderScale::FULL),
        arg:       Some(Arg { value_name: "FACTOR", ..Arg::long("render-scale") }),
        stability: Stability::Stable,
        since:     Since::This,
        group:     "display",
        example:   "--render-scale 0.75",
        reviewed:  Some(ReviewDate::parse("2026-07-21")),
        summary:   "Internal render scale before upscaling",
        ..Input::EMPTY
    };

    /// Three states, where a boolean would be a lie.
    ACCEL: Tristate = Input {
        key:       "accel",
        ty:        Type::Enum,
        default:   Some(Tristate::Auto),
        allowed:   Tristate::TOKENS,
        env:       Some(Env::new("MYAPP_ACCEL")),
        arg:       Some(Arg { value_name: "MODE", ..Arg::long("accel") }),
        stability: Stability::Stable,
        since:     Since::This,
        group:     "display",
        example:   "--accel auto",
        reviewed:  Some(ReviewDate::parse("2026-07-21")),
        summary:   "Hardware acceleration; auto enables it when available",
        ..Input::EMPTY
    };

    /// On its way out, pointing at its successor.
    PROFILE: bool = Input {
        key:       "profile",
        ty:        Type::Bool,
        default:   Some(false),
        env:       Some(Env::new("MYAPP_PROFILE")),
        arg:       Some(Arg::long("profile")),
        stability: Stability::Deprecated,
        since:     Since::This,
        deprecation: Some(Deprecation {
            since:       THIS_VERSION,
            replaced_by: Some(hud_replacement),
            migration:   "use --hud gpuload,frametimes instead",
        }),
        group:    "display",
        reviewed: Some(ReviewDate::parse("2026-07-21")),
        summary:  "Deprecated profiling overlay",
        ..Input::EMPTY
    };
}

fn main() {
    // One call: parse argv+env, lint the result, fail on errors.
    let resolved = Model::parse_and_lint(OnProblems::FailOnError);

    // Read typed values as often as needed — the resolution is cheap to clone.
    let level = Model::LOG_LEVEL.get_from_or_default(&resolved);
    let hdr = Model::HDR.get_from_or_default(&resolved);

    eprintln!(
        "log_level = {level:?} (from {:?}), hdr = {hdr:?} (from {:?})",
        resolved.source("log_level"),
        resolved.source("hdr")
    );

    // Emit the machine-readable contract — what `argenv lint` reads.
    println!(
        "{}",
        serde_json::to_string_pretty(&document("myapp@a1b2c3d", &Model::records())).unwrap()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn the_model_satisfies_every_rule() {
        assert!(Model::problems().is_empty(), "{:#?}", Model::problems());
    }

    #[test]
    fn an_argument_beats_an_environment_variable() {
        let env: BTreeMap<String, String> =
            [("MYAPP_LOG_LEVEL".to_string(), "error".to_string())].into();
        let args = vec!["--log-level".to_string(), "warn".to_string()];
        let r = Invocation {
            args: &args,
            env: &env,
        }
        .resolve(&Model::records());
        assert_eq!(Model::LOG_LEVEL.get_from(&r), Some(LogLevel::Warn));
        assert_eq!(r.source("log_level"), Some(Source::Arg));
    }
}
