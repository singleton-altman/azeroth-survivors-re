//! The survivors mode's UI strings, with Simplified Chinese entries and English fallback.
//!
//! The layer above this one (spell/zone/creature names) is locale-free: those names come out of
//! the client data's DBCs, so zhCN data localizes them on its own. What lives here is the mode's
//! own chrome — menus, the HUD, level-up and death screens — whose strings are authored in code.
//!
//! Language selection (v1): `WOW_LANG` (`webenv::var`, so `?lang=zhcn` on the web, the env var on
//! native), read once on first use. `zh…` picks Chinese, anything else (or unset) English. Sniffing
//! the data locale automatically is deliberately not done yet: the HUD's font handle is chosen when
//! the HUD spawns, and a mid-run flip would put Chinese text on an Latin-only font (tofu). v2 wants
//! a boot-time probe of the data chain (e.g. `Fonts\FZLBJW.TTF` only exists in zhCN installs).
//!
//! [`t`] translates bodyless strings; [`tf`] translates templates whose `{}`/`{name}` holes are
//! filled from the call site's values, and [`tr!`] wraps both so a call site's diff is one token:
//! `format!("Damage {} » {}", a, b)` becomes `tr!("Damage {} » {}", a, b)`. Untranslated keys fall
//! back to the English source, so new strings work before their entry lands.

use std::sync::OnceLock;

/// The UI language. `En` is the source language and always present; `ZhCn` is the first
/// translation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    ZhCn,
}

static LANG: OnceLock<Lang> = OnceLock::new();

/// The active language, decided once: `WOW_LANG` starting with `zh` picks [`Lang::ZhCn`], any
/// other value picks English, and unset stays English until something calls [`set_lang`].
pub fn lang() -> Lang {
    *LANG.get_or_init(|| match crate::webenv::var("WOW_LANG") {
        Some(v) if v.starts_with("zh") => Lang::ZhCn,
        Some(_) => Lang::En,
        None => Lang::En,
    })
}

/// Force the language. Call before the first UI string renders; later calls change nothing
/// (the font and already-spawned text were chosen on the earlier answer).
#[allow(dead_code)] // v2 的自动探测会用到;当前语言只来自 WOW_LANG
pub fn set_lang(lang: Lang) {
    let _ = LANG.set(lang);
}

/// True when `text` carries Han characters — the cheap locale probe for data-derived strings.
#[allow(dead_code)]
fn looks_chinese(text: &str) -> bool {
    text.chars()
        .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
}

/// Flip to Chinese the first time a data-derived string reads Chinese. v1 wiring point:
/// [`crate::survivors::hud`] calls this with DBC-resolved spell names, after the HUD's font
/// choice — a call before that spawn point would close the gap noted in the module docs.
#[allow(dead_code)]
pub fn sniff_data_locale(text: &str) {
    if LANG.get().is_none() && looks_chinese(text) {
        set_lang(Lang::ZhCn);
    }
}

/// English source → Simplified Chinese, sorted by key (the unit test pins the order).
static ZH: &[(&str, &str)] = &[
    ("Abandon run", "放弃本局"),
    ("Alliance", "联盟"),
    ("Area {} » {} yd", "范围 {} » {} 码"),
    ("Bosses", "首领"),
    ("Choose a Zone", "选择地区"),
    ("Choose your power", "选择你的力量"),
    ("Click a card, or press 1, 2 or 3", "点击卡牌，或按 1、2、3"),
    ("Common", "普通"),
    ("Cooldown {}s » {}s", "冷却 {} 秒 » {} 秒"),
    ("DPS", "DPS"),
    ("Damage", "伤害"),
    ("Damage meter", "伤害统计"),
    ("Damage taken", "承受伤害"),
    ("Damage {} » {}", "伤害 {} » {}"),
    ("Druid", "德鲁伊"),
    ("Dwarf", "矮人"),
    ("Enter World", "进入世界"),
    ("Epic", "史诗"),
    ("Everything is mastered. Restores all health.", "万法皆通。恢复所有生命值。"),
    ("Female", "女"),
    ("Gnome", "侏儒"),
    ("Healthstone", "治疗石"),
    ("Horde", "部落"),
    ("Human", "人类"),
    ("Hunter", "猎人"),
    ("Kills", "击杀"),
    ("Kills 0", "击杀 0"),
    ("Kills {kills}", "击杀 {kills}"),
    ("LEVEL UP", "升级"),
    ("Lasts {}s » {}s", "持续 {} 秒 » {} 秒"),
    ("Legendary", "传说"),
    ("Level", "等级"),
    ("Level 1", "等级 1"),
    ("Level {lo}", "等级 {lo}"),
    ("Level {lo}-{hi}", "等级 {lo}-{hi}"),
    ("Level {}", "等级 {}"),
    ("Mage", "法师"),
    ("Male", "男"),
    ("Night Elf", "暗夜精灵"),
    ("Orc", "兽人"),
    ("Paladin", "圣骑士"),
    ("Paused", "已暂停"),
    ("Peak DPS", "峰值 DPS"),
    ("Play again", "再来一局"),
    ("Priest", "牧师"),
    ("Quit game", "退出游戏"),
    ("Randomize", "随机"),
    ("Rank {level} » {}", "等级 {level} » {}"),
    ("Rank {} » {next}", "等级 {} » {next}"),
    ("Rare", "稀有"),
    ("Restore", "恢复"),
    ("Resume", "继续"),
    ("Rogue", "盗贼"),
    ("Shaman", "萨满"),
    ("Survived", "存活"),
    ("Targets {} » {}", "目标 {} » {}"),
    ("The horde was too much this time.", "这一战，部落实在太多了。"),
    ("Total damage", "总伤害"),
    ("Troll", "巨魔"),
    ("Uncommon", "优秀"),
    ("Undead", "亡灵"),
    ("Victory!", "胜利！"),
    ("Warlock", "术士"),
    ("Warrior", "战士"),
    ("XP {} / {}", "经验 {} / {}"),
    ("You have fallen", "你已阵亡"),
    ("cutting their damage by {} for {} sec", "使其造成的伤害降低 {}，持续 {} 秒"),
    ("freezing them in place for {} sec", "将其定身 {} 秒"),
    ("healing you for {} of the damage", "为你恢复所造成伤害的 {}"),
    ("killing them outright below {} health", "目标生命值低于 {} 时将其直接击杀"),
    ("melee", "自动攻击"),
    ("sending them fleeing for {} sec", "使其逃跑，持续 {} 秒"),
    ("slowing them by {} for {} sec", "使其速度降低 {}，持续 {} 秒"),
    ("stunning them for {} sec", "使其昏迷 {} 秒"),
    ("thorns", "荆棘"),
];

/// The Chinese entry for an English source string, or `en` itself when untranslated.
pub fn t(en: &'static str) -> &'static str {
    translate(lang(), en)
}

fn translate(lang: Lang, en: &'static str) -> &'static str {
    match lang {
        Lang::En => en,
        Lang::ZhCn => ZH
            .binary_search_by(|(k, _)| (*k).cmp(en))
            .map(|i| ZH[i].1)
            .unwrap_or(en),
    }
}

/// [`t`] for templates: `{}` holes fill from `args` in order, `{name}` holes fill from the
/// argument whose name matches, and an unfilled hole is left visible (a mis-modelled template
/// shows, like `strings::fill` upstream). Values arrive pre-stringified.
pub fn tf(en: &'static str, args: &[(&str, &str)]) -> String {
    let template = translate(lang(), en);
    let mut used = vec![false; args.len()];
    let mut out = String::with_capacity(template.len() + args.len() * 8);
    let bytes = template.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                out.push('{');
                i += 2;
                continue;
            }
            if let Some(end) = template[i..].find('}') {
                let hole = &template[i + 1..i + end];
                let (index, value) = if hole.is_empty() {
                    let index = used
                        .iter()
                        .position(|u| !u)
                        .unwrap_or(used.len().saturating_sub(1));
                    (index, args.get(index).map(|(_, v)| *v))
                } else {
                    match args.iter().position(|(name, _)| *name == hole) {
                        Some(index) => (index, args[index].1.into()),
                        None => (usize::MAX, None),
                    }
                };
                if index < used.len() {
                    used[index] = true;
                }
                match value {
                    Some(v) => out.push_str(v),
                    None => out.push_str(&template[i..i + end + 1]),
                }
                i += end + 1;
                continue;
            }
        }
        out.extend(template[i..].chars().next());
        i += template[i..].chars().next().unwrap().len_utf8();
    }
    out
}

/// The call-site wrapper: one token of diff per string, `format!` → `tr!`.
#[macro_export]
macro_rules! tr {
    ($en:expr) => {
        $crate::survivors::i18n::t($en)
    };
    ($en:expr, $($value:expr),+ $(,)?) => {
        $crate::survivors::i18n::tf($en, &[$((stringify!($value), &$value.to_string())),+])
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zh_table_is_sorted_for_binary_search() {
        assert!(ZH.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn untranslated_keys_fall_back_to_english() {
        assert_eq!(translate(Lang::En, "No such key"), "No such key");
        assert_eq!(
            translate(Lang::ZhCn, "No such key"),
            "No such key",
            "untranslated keys pass through in zh too"
        );
    }

    #[test]
    fn zh_translates_the_menu_strings() {
        assert_eq!(translate(Lang::ZhCn, "Enter World"), "进入世界");
        assert_eq!(translate(Lang::ZhCn, "Choose your power"), "选择你的力量");
        assert_eq!(translate(Lang::ZhCn, "LEVEL UP"), "升级");
    }

    #[test]
    fn tf_fills_sequential_and_named_holes() {
        assert_eq!(
            tf("Rank {} » {next}", &[("o.level", "3"), ("next", "4")]),
            "Rank 3 » 4"
        );
        assert_eq!(
            tf("Level {lo}-{hi}", &[("lo", "1"), ("hi", "5")]),
            "Level 1-5"
        );
        assert_eq!(tf("XP {} / {}", &[("a", "10"), ("b", "20")]), "XP 10 / 20");
    }

    #[test]
    fn sequential_and_named_holes_both_fill() {
        assert_eq!(
            translate(Lang::ZhCn, "Rank {} » {next}")
                .replace("{next}", "4")
                .replace("{}", "3"),
            "等级 3 » 4"
        );
        let s = translate(Lang::ZhCn, "Level {lo}-{hi}")
            .replace("{lo}", "1")
            .replace("{hi}", "5");
        assert_eq!(s, "等级 1-5");
    }
}
