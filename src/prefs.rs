/// How a merge should be ordered. This is the structured stub beside plain English.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bias {
    Balanced,
    FavorMain,
    FavorRamp,
}

impl std::fmt::Display for Bias {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Bias::Balanced => "balanced",
            Bias::FavorMain => "favor_main",
            Bias::FavorRamp => "favor_ramp",
        })
    }
}

/// Preferences Muse is allowed to see. `structured` is optional and wins when it is set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prefs {
    pub english: String,
    pub structured: Option<Bias>,
}

impl Prefs {
    pub fn new(english: impl Into<String>) -> Self {
        Self {
            english: english.into(),
            structured: None,
        }
    }

    pub fn with_bias(english: impl Into<String>, bias: Bias) -> Self {
        Self {
            english: english.into(),
            structured: Some(bias),
        }
    }
}

/// Small keyword reader for the demo sentences. It is not a general parser.
pub fn interpret_english(english: &str) -> Bias {
    let text = english.to_lowercase();
    const FAIR: &[&str] = &[
        "fair",
        "zipper",
        "balance",
        "balanced",
        "equal share",
        "both approaches",
        "nobody waits forever",
    ];
    if FAIR.iter().any(|marker| text.contains(marker)) {
        return Bias::Balanced;
    }
    match (favors(&text, "main"), favors(&text, "ramp")) {
        (true, false) => Bias::FavorMain,
        (false, true) => Bias::FavorRamp,
        _ => Bias::Balanced,
    }
}

fn favors(text: &str, lane: &str) -> bool {
    let patterns = [
        format!("{lane} first"),
        format!("{lane} has priority"),
        format!("{lane} priority"),
        format!("{lane} road has priority"),
        format!("{lane} road priority"),
        format!("priority to the {lane}"),
        format!("priority to {lane}"),
        format!("favor {lane}"),
        format!("favor the {lane}"),
        format!("prioritize {lane}"),
        format!("prioritize the {lane}"),
        format!("prioritise {lane}"),
        format!("{lane} before"),
        format!("{lane} ahead"),
    ];
    patterns.iter().any(|pattern| text.contains(pattern))
}

/// Read the prefs block that was embedded in a Muse prompt.
pub fn bias_from_prefs_block(block: &str) -> Bias {
    let mut english = String::new();
    let mut structured = None;
    for line in block.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();
        if let Some(rest) = lower.strip_prefix("structured:") {
            structured = match rest.trim() {
                "favor_main" | "main" => Some(Bias::FavorMain),
                "favor_ramp" | "ramp" => Some(Bias::FavorRamp),
                "balanced" | "fair" => Some(Bias::Balanced),
                _ => None,
            };
        } else if let Some(rest) = strip_english_prefix(trimmed) {
            english = rest.to_string();
        }
    }
    if let Some(bias) = structured {
        return bias;
    }
    if english.is_empty() {
        interpret_english(block)
    } else {
        interpret_english(&english)
    }
}

fn strip_english_prefix(line: &str) -> Option<&str> {
    const PREFIX: &str = "english:";
    if line.len() >= PREFIX.len() && line[..PREFIX.len()].eq_ignore_ascii_case(PREFIX) {
        Some(line[PREFIX.len()..].trim())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fair_sentences_are_balanced() {
        assert_eq!(
            interpret_english("Zipper fairly so both approaches get home."),
            Bias::Balanced
        );
    }

    #[test]
    fn priority_sentences_favor_one_approach() {
        assert_eq!(
            interpret_english("Main has priority. Main first."),
            Bias::FavorMain
        );
        assert_eq!(
            interpret_english("Ramp has priority. Ramp first."),
            Bias::FavorRamp
        );
    }

    #[test]
    fn structured_line_wins_over_fair_english() {
        let block = "english: Zipper fairly so both approaches get home.\nstructured: favor_main\n";
        assert_eq!(bias_from_prefs_block(block), Bias::FavorMain);
    }
}
