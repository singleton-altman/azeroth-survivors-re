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

use bevy::prelude::*;
use bevy::asset::LoadState;

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

/// v2 自动探测:开机加载 `Fonts\FZLBJW.TTF`——只有 zhCN 安装的数据链里存在这个文件,
/// 加载成功即把语言切到中文,失败(enUS 等)保持英文并停止探测。
/// `WOW_LANG` 显式设置时跳过探测(显式优先)。在探测翻转前就已生成的文本保持原语言。
pub fn probe_data_locale(
    server: Res<AssetServer>,
    mut probe: Local<Option<Handle<Font>>>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    if crate::webenv::var("WOW_LANG").is_some() {
        *done = true;
        return;
    }
    let handle = probe.get_or_insert_with(|| server.load("mpq://Fonts/FZLBJW.TTF"));
    match server.get_load_state(handle.id()) {
        Some(LoadState::Loaded) => {
            set_lang(Lang::ZhCn);
            *done = true;
        }
        Some(LoadState::Failed(_)) => *done = true,
        _ => {}
    }
}

/// English source → Simplified Chinese, sorted by key (the unit test pins the order).
static ZH: &[(&str, &str)] = &[
    (", and everything within {} yd of the hit", "，并波及命中点 {} 码内的一切"),
    (", then {} damage every second for {} sec", "，随后每秒造成 {} 点伤害，持续 {} 秒"),
    ("Abandon run", "放弃本局"),
    ("Absorbs the next {d} damage you take within {} sec", "吸收 {} 秒内你受到的下 {d} 点伤害"),
    ("Afflicts {} enemies in range with {} damage every second for {} sec", "使范围内 {} 个敌人每秒受到 {} 点伤害，持续 {} 秒"),
    ("All damage", "所有伤害"),
    ("Alliance", "联盟"),
    ("Anub'Rekhan", "阿努布雷坎"),
    ("Anubisath Sentinel", "阿努比萨斯哨兵"),
    ("Area size", "范围大小"),
    ("Area {} » {} yd", "范围 {} » {} 码"),
    ("Aspect of the Cheetah", "猎豹守护"),
    ("Below 85% health, heals you for {d} over {} sec", "生命值低于 85% 时，在 {} 秒内为你恢复 {d} 点生命值"),
    ("Bjarn", "巴尔恩"),
    ("Blessing of Freedom", "自由祝福"),
    ("Blink", "闪现术"),
    ("Bosses", "首领"),
    ("Buru the Gorger", "暴食者布鲁"),
    ("Casts itself", "自动施放"),
    ("Charge", "冲锋"),
    ("Choose a Zone", "选择地区"),
    ("Choose your power", "选择你的力量"),
    ("Click a card, or press 1, 2 or 3", "点击卡牌，或按 1、2、3"),
    ("Common", "普通"),
    ("Cooldown {}s » {}s", "冷却 {} 秒 » {} 秒"),
    ("Covers a {} yd area {at} for {} sec, dealing {d} damage every second", "覆盖{at}的 {} 码范围，持续 {} 秒，每秒造成 {d} 点伤害"),
    ("Critical strike chance", "暴击几率"),
    ("DPS", "DPS"),
    ("Damage", "伤害"),
    ("Damage blocked per hit", "每次受击减免的伤害"),
    ("Damage meter", "伤害统计"),
    ("Damage taken", "承受伤害"),
    ("Damage {} » {}", "伤害 {} » {}"),
    ("Dash", "疾奔"),
    ("Deals {d} damage to every enemy in a {} yd cone toward the closest", "朝最近敌人方向释出 {} 码锥形范围，对其中每个敌人造成 {d} 点伤害"),
    ("Deals {d} damage to every enemy within {} yd of you", "对你周围 {} 码内的每个敌人造成 {d} 点伤害"),
    ("Death Lord", "死亡领主"),
    ("Devilsaur", "魔暴龙"),
    ("Doomguard Commander", "末日守卫指挥官"),
    ("Drops a totem for {} sec that burns every enemy within {} yd for {d} damage every 2 sec", "放置一根图腾，持续 {} 秒，每 2 秒对 {} 码内每个敌人造成 {d} 点烈焰伤害"),
    ("Drops a totem for {} sec that hits the nearest enemy within {} yd for {d} damage every second", "放置一根图腾，持续 {} 秒，每秒对 {} 码内最近的敌人造成 {d} 点伤害"),
    ("Druid", "德鲁伊"),
    ("Dun Morogh", "丹莫罗"),
    ("Dun Morogh thaws in your wake.", "在你走过之后，丹莫罗冰雪消融。"),
    ("Durotar", "杜隆塔尔"),
    ("Durotar is quiet again.", "杜隆塔尔重归宁静。"),
    ("Duskwood", "暮色森林"),
    ("Duskwood's curse lifts.", "暮色森林的诅咒消散了。"),
    ("Dwarf", "矮人"),
    ("Elwynn Forest", "艾尔文森林"),
    ("Elwynn Forest is safe once more.", "艾尔文森林重归安宁。"),
    ("Emeriss", "艾莫莉丝"),
    ("Enter World", "进入世界"),
    ("Epic", "史诗"),
    ("Even King Mosh has fallen.", "连莫什王也已倒下。"),
    ("Every {} sec, deals {d} damage to every enemy within {} yd of you", "每 {} 秒，对你周围 {} 码内的每个敌人造成 {d} 点伤害"),
    ("Every {} sec, zaps {} within {} yd of you for {d} damage", "每 {} 秒，对你周围 {} 码内的 {} 释放闪电，造成 {d} 点伤害"),
    ("Everything is mastered. Restores all health.", "万法皆通。恢复所有生命值。"),
    ("Experience gained", "获得经验"),
    ("Fade", "渐隐术"),
    ("Female", "女"),
    ("Fire Plume Ridge", "火羽岭"),
    ("Gates of Ahn'Qiraj", "安其拉之门"),
    ("Ghost Wolf", "幽魂之狼"),
    ("Gluth", "格拉斯"),
    ("Gnome", "侏儒"),
    ("Gurubashi Arena", "古拉巴什竞技场"),
    ("Gurubashi Berserker", "古拉巴什狂暴者"),
    ("Hakkar", "哈卡"),
    ("Harvest Golem", "收割傀儡"),
    ("Harvest Watcher", "农耕收割者"),
    ("Health per second", "每秒恢复生命"),
    ("Healthstone", "治疗石"),
    ("Highest rank", "已达最高等级"),
    ("Hits", "命中"),
    ("Horde", "部落"),
    ("Howl of Terror", "恐惧嚎叫"),
    ("Human", "人类"),
    ("Hunter", "猎人"),
    ("Ice Claw Bear", "冰爪熊"),
    ("Increases all damage you deal by {}.", "你造成的所有伤害提高 {}。"),
    ("Increases the experience you gain by {}.", "你获得的经验提高 {}。"),
    ("Increases the size of your areas of effect by {}.", "你的效果范围扩大 {}。"),
    ("Increases your chance to critically strike for double damage by {}.", "你造成双倍伤害暴击的几率提高 {}。"),
    ("Increases your maximum health by {}.", "你的最大生命值提高 {}。"),
    ("Increases your movement speed by {}.", "你的移动速度提高 {}。"),
    ("Increases your pickup range by {}.", "你的拾取范围扩大 {}。"),
    ("Instant", "瞬发"),
    ("Kel'Thuzad's necropolis falls dark.", "克尔苏加德的纳克萨玛斯陷入黑暗。"),
    ("Kills", "击杀"),
    ("Kills 0", "击杀 0"),
    ("Kills {kills}", "击杀 {kills}"),
    ("King Bangalash", "班加拉什王"),
    ("King Mosh", "莫什王"),
    ("Kurinnaxx", "库林纳克斯"),
    ("LEVEL UP", "升级"),
    ("Lasts {}s » {}s", "持续 {} 秒 » {} 秒"),
    ("Legendary", "传说"),
    ("Lesser Infernal", "次级地狱火"),
    ("Lethon", "莱索恩"),
    ("Level", "等级"),
    ("Level 1", "等级 1"),
    ("Level {lo}", "等级 {lo}"),
    ("Level {lo}-{hi}", "等级 {lo}-{hi}"),
    ("Level {}", "等级 {}"),
    ("Lord Kazzak", "卡扎克领主"),
    ("Mage", "法师"),
    ("Male", "男"),
    ("Maximum health", "最大生命值"),
    ("Melee Range", "近战范围"),
    ("Mount Hyjal", "海加尔山"),
    ("Movement speed", "移动速度"),
    ("Mulgore", "莫高雷"),
    ("Mulgore's herds roam free.", "莫高雷的兽群自由漫步。"),
    ("Naxxramas", "纳克萨玛斯"),
    ("Next rank:", "下一等级："),
    ("Night Elf", "暗夜精灵"),
    ("Nordrassil endures.", "诺达希尔巍然长存。"),
    ("Obsidian Destroyer", "黑曜石毁灭者"),
    ("Orc", "兽人"),
    ("Paladin", "圣骑士"),
    ("Patchwerk", "帕奇维克"),
    ("Paused", "已暂停"),
    ("Peak DPS", "峰值 DPS"),
    ("Pickup range", "拾取范围"),
    ("Plated Stegodon", "铁甲剑龙"),
    ("Play again", "再来一局"),
    ("Priest", "牧师"),
    ("Princess Huhuran", "哈霍兰公主"),
    ("Qiraji Champion", "其拉勇士"),
    ("Quit game", "退出游戏"),
    ("Randomize", "随机"),
    ("Rank {level} » {}", "等级 {level} » {}"),
    ("Rank {}", "等级 {}"),
    ("Rank {} » {next}", "等级 {} » {next}"),
    ("Rare", "稀有"),
    ("Recharge speed", "充能速度"),
    ("Redridge Mountains", "赤脊山"),
    ("Redridge is safe once more.", "赤脊山重归安宁。"),
    ("Reduces the damage of every hit you take by {}.", "你每次受到的伤害降低 {}。"),
    ("Restore", "恢复"),
    ("Restores {} health every second.", "每秒恢复 {} 点生命值。"),
    ("Resume", "继续"),
    ("Rogue", "盗贼"),
    ("Shaman", "萨满"),
    ("Slaughterhouse Protector", "屠宰场守护者"),
    ("Sprint", "疾跑"),
    ("Stegodon", "剑龙"),
    ("Stitched Horror", "缝合恐怖兽"),
    ("Stone Guardian", "石头守护者"),
    ("Strikes", "打击"),
    ("Strikes a {} yd area {at} for {d} damage", "打击{at}的 {} 码范围，造成 {d} 点伤害"),
    ("Strikes the nearest enemy for {d} damage, then jumps to {} more within {} yd of each other, 15% weaker with every jump", "打击距离最近的敌人，造成 {d} 点伤害，随后跳向 {} 个 {} 码内的其他目标，每次跳跃伤害降低 15%"),
    ("Survived", "存活"),
    ("Taerar", "泰拉尔"),
    ("Targets {} » {}", "目标 {} » {}"),
    ("Teldrassil", "泰达希尔"),
    ("Teldrassil blooms again.", "泰达希尔再度繁花盛开。"),
    ("Teremus the Devourer", "吞噬者特雷姆斯"),
    ("Thaddius", "塔迪乌斯"),
    ("The Barrens", "贫瘠之地"),
    ("The Barrens are yours.", "贫瘠之地已归于你。"),
    ("The Dark Portal", "黑暗之门"),
    ("The Dark Portal stands silent.", "黑暗之门重归寂静。"),
    ("The Qiraji retreat behind the Scarab Wall.", "其拉虫人退回了甲虫之墙之后。"),
    ("The arena roars your name.", "竞技场高呼你的名字。"),
    ("The horde was too much this time.", "这一战，部落实在太多了。"),
    ("Tirisfal Glades", "提瑞斯法林地"),
    ("Tirisfal is at peace.", "提瑞斯法重归安息。"),
    ("Total damage", "总伤害"),
    ("Troll", "巨魔"),
    ("Tyrant Devilsaur", "暴虐魔暴龙"),
    ("Uncommon", "优秀"),
    ("Undead", "亡灵"),
    ("Victory!", "胜利！"),
    ("Warlock", "术士"),
    ("Warrior", "战士"),
    ("Westfall", "西部荒野"),
    ("Westfall's fields are safe.", "西部荒野的田野重归安全。"),
    ("When an enemy closes in, makes you immune to all damage for {} sec", "当敌人靠近时，使你免疫所有伤害，持续 {} 秒"),
    ("When enemies close in, deals {d} damage to every attacker for {} sec", "当敌人靠近时，对每个攻击者造成 {d} 点伤害，持续 {} 秒"),
    ("XP {} / {}", "经验 {} / {}"),
    ("You have fallen", "你已阵亡"),
    ("You have learned {}! Press F.", "你学会了 {}！按 F 使用。"),
    ("Young Black Bear", "幼黑熊"),
    ("Young Forest Bear", "幼森林熊"),
    ("Your abilities recharge {} faster.", "你的技能充能速度加快 {}。"),
    ("Ysondre", "伊索德雷"),
    ("absorbed", "吸收"),
    ("an enemy", "一个敌人"),
    ("around you", "你周围"),
    ("cutting their damage by {} for {} sec", "使其造成的伤害降低 {}，持续 {} 秒"),
    ("damage", "伤害"),
    ("freezing them in place for {} sec", "将其定身 {} 秒"),
    ("healing", "治疗"),
    ("healing you for {} of the damage", "为你恢复所造成伤害的 {}"),
    ("killing them outright below {} health", "目标生命值低于 {} 时将其直接击杀"),
    ("melee", "自动攻击"),
    ("on the thickest pack", "敌人最密集处"),
    ("sending them fleeing for {} sec", "使其逃跑，持续 {} 秒"),
    ("slowing them by {} for {} sec", "使其速度降低 {}，持续 {} 秒"),
    ("stunning them for {} sec", "使其昏迷 {} 秒"),
    ("the closest enemy", "最近的敌人"),
    ("the nearest enemy", "距离最近的敌人"),
    ("the toughest enemy in range", "范围内最强的敌人"),
    ("the {n} closest enemies", "最近的 {n} 个敌人"),
    ("the {n} nearest enemies", "距离最近的 {n} 个敌人"),
    ("the {n} toughest enemies in range", "范围内 {n} 个最强的敌人"),
    ("thorns", "荆棘"),
    ("{} approaches!", "{} 来袭！"),
    ("{} chance to critically strike for double damage", "有 {} 几率造成双倍伤害的暴击"),
    ("{} damage per second", "每秒 {} 伤害"),
    ("{} enemies", "{} 个敌人"),
    ("{} jumps", "{} 跳"),
    ("{} sec", "{} 秒"),
    ("{} sec cooldown", "冷却 {} 秒"),
    ("{} targets", "{} 个目标"),
    ("{} yd", "{} 码"),
    ("{} yd range", "{} 码射程"),
    ("{} {who} for {d} damage", "{verb}{who}，造成 {d} 点伤害"),
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
