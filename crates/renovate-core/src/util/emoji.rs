use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

static UNICODE_EMOJI_MAP: &[(&str, &str)] = &[
    (":warning:", "\u{26a0}\u{fe0f}"),
    (":date:", "\u{1f4c5}"),
    (":vertical_traffic_light:", "\u{1f6a6}"),
    (":recycle:", "\u{267b}\u{fe0f}"),
    (":ghost:", "\u{1f47b}"),
    (":no_bell:", "\u{1f515}"),
    (":tada:", "\u{1f389}"),
    (":rocket:", "\u{1f680}"),
    (":lock:", "\u{1f512}"),
    (":rotating_light:", "\u{1f6a8}"),
    (":white_check_mark:", "\u{2705}"),
    (":x:", "\u{274c}"),
    (":heavy_check_mark:", "\u{2714}\u{fe0f}"),
    (":book:", "\u{1f4d6}"),
    (":mag:", "\u{1f50d}"),
    (":bug:", "\u{1f41b}"),
    (":hammer:", "\u{1f528}"),
    (":star:", "\u{2b50}"),
    (":heart:", "\u{2764}\u{fe0f}"),
    (":hand:", "\u{270b}"),
    (":eyes:", "\u{1f440}"),
    (":fire:", "\u{1f525}"),
    (":boom:", "\u{1f4a5}"),
    (":zap:", "\u{26a1}"),
    (":package:", "\u{1f4e6}"),
    (":gear:", "\u{2699}\u{fe0f}"),
    (":wrench:", "\u{1f527}"),
    (":arrow_up:", "\u{2b06}\u{fe0f}"),
    (":arrow_down:", "\u{2b07}\u{fe0f}"),
    (":arrow_right:", "\u{27a1}\u{fe0f}"),
    (":leftwards_arrow_with_hook:", "\u{21a9}\u{fe0f}"),
    (":see_no_evil:", "\u{1f648}"),
    (":truck:", "\u{1f69a}"),
    (":construction:", "\u{1f6a7}"),
    (":green_heart:", "\u{1f49a}"),
    (":broken_heart:", "\u{1f494}"),
    (":bell:", "\u{1f514}"),
    (":bookmark:", "\u{1f516}"),
    (":link:", "\u{1f517}"),
    (":bulb:", "\u{1f4a1}"),
    (":construction_worker:", "\u{1f477}"),
    (":whale:", "\u{1f433}"),
    (":snake:", "\u{1f40d}"),
    (":crab:", "\u{1f980}"),
    (":elephant:", "\u{1f418}"),
    (":monkey:", "\u{1f412}"),
    (":bird:", "\u{1f426}"),
    (":penguin:", "\u{1f427}"),
    (":beetle:", "\u{1f41e}"),
    (":ant:", "\u{1f41c}"),
    (":bee:", "\u{1f41d}"),
    (":snail:", "\u{1f40c}"),
    (":bug:", "\u{1f41b}"),
    (":chart_with_upwards_trend:", "\u{1f4c8}"),
    (":chart_with_downwards_trend:", "\u{1f4c9}"),
];

/// Convert all known emoji shortcodes in a message body to Unicode.
/// @parity lib/util/emoji.ts full
pub fn emojify(text: &str) -> String {
    if !use_unicode_emoji() {
        return text.to_owned();
    }
    let mut result = text.to_owned();
    let emoji_map: HashMap<&str, &str> = UNICODE_EMOJI_MAP.iter().copied().collect();
    for (code, unicode) in &emoji_map {
        result = result.replace(code, unicode);
    }
    result
}

static USE_UNICODE_EMOJI: AtomicBool = AtomicBool::new(true);

pub fn set_emoji_config(unicode: bool) {
    USE_UNICODE_EMOJI.store(unicode, Ordering::Relaxed);
}

fn use_unicode_emoji() -> bool {
    USE_UNICODE_EMOJI.load(Ordering::Relaxed)
}

pub fn get_emoji(code: &str) -> Option<&'static str> {
    UNICODE_EMOJI_MAP
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, u)| *u)
}

pub fn unicode_emoji_map() -> &'static [(&'static str, &'static str)] {
    UNICODE_EMOJI_MAP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emojify_replaces_codes() {
        assert_eq!(emojify(":rocket:"), "\u{1f680}");
    }

    #[test]
    fn emojify_multiple_codes() {
        let result = emojify(":rocket: :star:");
        assert!(result.contains('\u{1f680}'));
        assert!(result.contains('\u{2b50}'));
        assert!(!result.contains(":rocket:"));
    }

    #[test]
    fn emojify_no_codes() {
        assert_eq!(emojify("plain text"), "plain text");
    }

    #[test]
    fn emojify_empty() {
        assert_eq!(emojify(""), "");
    }

    #[test]
    fn emojify_unknown_code_unchanged() {
        assert_eq!(emojify(":unknown_emoji:"), ":unknown_emoji:");
    }

    #[test]
    fn get_emoji_found() {
        assert_eq!(get_emoji(":rocket:"), Some("\u{1f680}"));
    }

    #[test]
    fn get_emoji_not_found() {
        assert_eq!(get_emoji(":nonexistent:"), None);
    }

    #[test]
    fn unicode_emoji_map_not_empty() {
        assert!(!unicode_emoji_map().is_empty());
    }
}
