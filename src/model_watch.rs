//! Check the Ollama library for newer releases of the model families the user
//! already has pulled locally, evaluate them against the local VRAM budget,
//! and recommend upgrades.
//!
//! This mirrors the manual workflow of browsing `ollama.com/library/<family>`
//! and comparing the available tags (e.g. `qwen3-coder:30b`, `480b`) against
//! what is currently pulled. It is intentionally network-tolerant: if a library
//! page cannot be fetched (offline, or a JS-rendered shell like Hunyuan), that
//! family is reported as "could not check" rather than failing the whole run.

use crate::Platform;
use regex::Regex;
use std::sync::OnceLock;

/// DevOps-relevant Ollama library repos to watch. A pulled local model is
/// mapped to one of these by prefix match so we only check families the user
/// actually uses.
const WATCH_FAMILIES: &[&str] = &[
    "qwen3-coder",
    "qwen3",
    "qwen2.5-coder",
    "gemma4",
    "gemma3",
    "gemma2",
    "deepseek-r1",
    "deepseek-coder",
    "deepseek-v3",
    "codegemma",
    "devstral",
    "hunyuan",
    "llama3.3",
    "llama3.2",
    "llama3.1",
    "mistral",
    "mixtral",
    "phi4",
    "granite-code",
    "command-r",
    "starling",
    "wizardcoder",
    "nomic-embed-text",
];

#[derive(Debug, Clone)]
pub struct FamilyCheck {
    /// Ollama library repo, e.g. `qwen3-coder`.
    pub family: String,
    /// Full local model name currently pulled, if any.
    pub current_model: Option<String>,
    /// Size token of the local model, e.g. `30b`.
    pub current: Option<String>,
    /// Tags observed in the Ollama library for this family.
    pub available: Vec<String>,
    /// Largest available variant token (by parameter count).
    pub latest: Option<String>,
    /// Estimated q4 size of the latest variant, in GB.
    pub latest_est_gb: f32,
    /// Whether the latest variant is estimated to fit the local VRAM budget.
    pub fits: bool,
    /// Human-readable recommendation.
    pub recommendation: String,
}

/// Approximate VRAM (GB) for a single-GPU profile, used only to estimate
/// whether a candidate model would fit. Conservative by design.
pub fn platform_vram_gb(platform: Platform) -> u32 {
    match platform {
        Platform::MacOSM424Gb | Platform::MacOSM524Gb => 24,
        Platform::MacOSM432Gb | Platform::MacOSM532Gb => 32,
        // CachyOS / generic Linux single-GPU devops profile is a 24GB RTX 4090.
        _ => 24,
    }
}

/// Map the pulled local models to the Ollama library families we should watch.
pub fn watch_families_for(models: &[String]) -> Vec<String> {
    let mut fams = Vec::new();
    for m in models {
        for f in WATCH_FAMILIES {
            if m.starts_with(f) {
                if !fams.iter().any(|x: &String| x == f) {
                    fams.push(f.to_string());
                }
                break;
            }
        }
    }
    fams
}

/// Extract a parameter-size token (e.g. `30b`, `7b`, `2m`) from a model/tag
/// string. Returns the parameter count in billions.
fn extract_size_token(s: &str) -> Option<u32> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(?i)(\d+)\s*([bm])").unwrap());
    re.captures(s).map(|c| {
        let n: u32 = c[1].parse().unwrap_or(0);
        let unit = c[2].to_ascii_lowercase();
        if unit == "m" {
            // millibillions? treat 'm' as millions of params -> ~0 for our scale
            n / 1000
        } else {
            n
        }
    })
}

/// Known real q4 sizes (GB) for common tags, where the naive param-count
/// heuristic is wrong (e.g. `gemma4:31b` is ~62 GB, not ~17 GB). Keyed by
/// `<family>:<tag>`. Falls back to the param-count heuristic for unknown tags.
const KNOWN_SIZES_GB: &[(&str, f32)] = &[
    ("gemma4:12b", 8.0),
    ("gemma4:26b", 17.0),
    ("gemma4:31b", 62.0),
    ("qwen3-coder:30b", 18.0),
    ("qwen3-coder:480b", 260.0),
    ("deepseek-r1:32b", 20.0),
    ("deepseek-r1:70b", 43.0),
    ("deepseek-r1:671b", 400.0),
    ("devstral:24b", 13.0),
    ("codegemma:2b", 1.5),
    ("codegemma:7b", 5.0),
];

/// Estimate a model's VRAM footprint (GB). Uses a known-size override when
/// available, otherwise a rough q4 heuristic (~0.55 GB per billion params).
fn est_size_gb(family: &str, tag: &str) -> f32 {
    let key = format!("{}:{}", family, tag);
    for (k, v) in KNOWN_SIZES_GB {
        if *k == key {
            return *v;
        }
    }
    extract_size_token(tag)
        .map(|p| p as f32 * 0.55)
        .unwrap_or(0.0)
}

/// Fetch the available tags for an Ollama library family by scraping the
/// `ollama.com/library/<family>` page. Returns an empty list if the page
/// cannot be fetched or contains no recognisable tags (JS-rendered shell).
pub fn fetch_library_tags(family: &str) -> Vec<String> {
    let url = format!("https://ollama.com/library/{}", family);
    let body = match reqwest::blocking::get(&url).and_then(|r| r.text()) {
        Ok(b) => b,
        Err(_) => return vec![],
    };
    let pattern = format!(
        r"(?i){}(?::|%3A)([0-9a-z][0-9a-z.\-]*)",
        regex::escape(family)
    );
    let re = match Regex::new(&pattern) {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    let mut tags: Vec<String> = re
        .captures_iter(&body)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .filter(|t| !t.is_empty())
        .collect();
    tags.sort();
    tags.dedup();
    tags
}

/// Core function: for each family the user has pulled locally, check the
/// Ollama library for newer releases and produce an upgrade recommendation.
pub fn check_updates(
    base_url: &str,
    _platform: Platform,
    vram_gb: u32,
) -> anyhow::Result<Vec<FamilyCheck>> {
    let local = crate::kilo_integration::fetch_available_models(base_url);
    let families = watch_families_for(&local);
    let mut out = Vec::new();

    for fam in families {
        let cur_full = local.iter().find(|m| m.starts_with(&fam)).cloned();
        let cur_params = cur_full.as_ref().and_then(|m| extract_size_token(m));

        let tags = fetch_library_tags(&fam);
        let mut tagged: Vec<(String, u32)> = tags
            .iter()
            .filter_map(|t| extract_size_token(t).map(|s| (t.clone(), s)))
            .collect();
        tagged.sort_by(|a, b| b.1.cmp(&a.1));
        let latest = tagged.first().map(|(t, _)| t.clone());
        let latest_params = tagged.first().map(|(_, s)| *s).unwrap_or(0);
        let latest_gb = latest
            .as_deref()
            .map(|t| est_size_gb(&fam, t))
            .unwrap_or(0.0);
        let fits = latest_gb <= vram_gb as f32 - 1.0;

        let recommendation = if tags.is_empty() {
            format!(
                "could not check library (offline or JS-rendered page for '{}')",
                fam
            )
        } else {
            match (cur_full.is_some(), cur_params, latest.as_ref()) {
                (_, Some(c), Some(l)) if c >= latest_params => {
                    format!(
                        "up to date — '{}:{}' is the latest size in this family",
                        fam, l
                    )
                }
                (_, Some(c), Some(l)) if latest_params > c && fits => format!(
                    "upgrade available: {}:{} (~{:.0} GB) fits your {} GB VRAM — optional pull",
                    fam, l, latest_gb, vram_gb
                ),
                (_, Some(c), Some(l)) if latest_params > c && !fits => format!(
                    "newer available: {}:{} (~{:.0} GB) but exceeds your {} GB VRAM — skipped",
                    fam, l, latest_gb, vram_gb
                ),
                (true, None, Some(l)) => format!(
                    "present locally (untagged variant); latest is {} (~{:.0} GB){}",
                    l,
                    latest_gb,
                    if fits {
                        " — optional pull"
                    } else {
                        " — too large"
                    }
                ),
                (false, _, Some(l)) => format!(
                    "no local '{}' model pulled; latest is {} (~{:.0} GB){}",
                    fam,
                    l,
                    latest_gb,
                    if fits {
                        " — could add"
                    } else {
                        " — too large"
                    }
                ),
                _ => "no newer release detected".to_string(),
            }
        };

        out.push(FamilyCheck {
            family: fam,
            current_model: cur_full,
            current: cur_params.map(|n| format!("{}b", n)),
            available: tags,
            latest,
            latest_est_gb: latest_gb,
            fits,
            recommendation,
        });
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_token_from_model_names() {
        assert_eq!(extract_size_token("qwen3-coder:30b-gpu"), Some(30));
        assert_eq!(extract_size_token("gemma4-26b-devops"), Some(26));
        assert_eq!(extract_size_token("deepseek-r1:32b"), Some(32));
        // no size token -> None
        assert_eq!(extract_size_token("devstral-small-2-gpu"), None);
        assert_eq!(extract_size_token("nomic-embed-text"), None);
    }

    #[test]
    fn known_size_overrides_win_over_heuristic() {
        // gemma4:31b is ~62 GB, NOT the naive 31*0.55 ≈ 17 GB.
        assert!((est_size_gb("gemma4", "31b") - 62.0).abs() < 0.01);
        assert!((est_size_gb("gemma4", "26b") - 17.0).abs() < 0.01);
        assert!((est_size_gb("qwen3-coder", "30b") - 18.0).abs() < 0.01);
        assert!((est_size_gb("deepseek-r1", "32b") - 20.0).abs() < 0.01);
        // unknown tag falls back to heuristic
        assert!((est_size_gb("foo", "99b") - 54.45).abs() < 0.01);
    }

    #[test]
    fn watch_families_only_known_prefixes_of_pulled() {
        let models = vec![
            "qwen3-coder:30b-gpu".to_string(),
            "gemma4-26b-devops".to_string(),
            "devstral-small-2-gpu".to_string(),
            "nomic-embed-text".to_string(),
        ];
        let fams = watch_families_for(&models);
        assert!(fams.contains(&"qwen3-coder".to_string()));
        assert!(fams.contains(&"gemma4".to_string()));
        assert!(fams.contains(&"devstral".to_string()));
        assert!(fams.contains(&"nomic-embed-text".to_string()));
        // unrelated pulled model should not add a watched family
        let only_unrelated = vec!["some-other-model".to_string()];
        assert!(watch_families_for(&only_unrelated).is_empty());
    }
}
