//! 伤害统计悬浮窗:总伤害、5 秒滚动 DPS、峰值、承受伤害与按法术的排行。
//! `V` 键开合;仅在 [`Phase::Playing`](super::Phase) 显示。文案走 [`i18n`]。

use bevy::prelude::*;
use benilla_world::schedule::WorldStage;

use super::{abilities, data, hud, i18n, Phase, Run};

/// 主面板根节点。
#[derive(Component)]
struct MeterWindow;

/// 面板上的四行数值。
#[derive(Component, Clone, Copy, PartialEq)]
enum MeterField {
    Total,
    Dps,
    Peak,
    Taken,
}

/// 排行榜的第 `0` 行(0 = 最高)。
#[derive(Component)]
struct MeterTopLine(usize);

#[derive(Resource, Default)]
pub(super) struct MeterOpen(bool);

pub(super) struct DamageMeterPlugin;

impl Plugin for DamageMeterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup.after(hud::setup_hud))
            .add_systems(
                Update,
                (toggle, refresh.after(abilities::apply_hits)).after(WorldStage::Input),
            );
    }
}

fn px(v: f32) -> Val {
    Val::Px(v)
}

fn text(
    font: &Handle<Font>,
    s: impl Into<String>,
    size: f32,
    color: Color,
) -> (Text, TextFont, TextColor) {
    (
        Text::new(s),
        TextFont {
            font: font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
    )
}

const GOLD: Color = Color::srgb(1.0, 0.82, 0.25);
const WHITE: Color = Color::WHITE;
const DIM: Color = Color::srgb(0.62, 0.62, 0.62);

fn setup(mut commands: Commands, font: Res<hud::HudFont>) {
    let f = font.0.clone();
    let window = Node {
        position_type: PositionType::Absolute,
        left: px(12.0),
        top: px(150.0),
        width: px(252.0),
        flex_direction: FlexDirection::Column,
        row_gap: px(4.0),
        padding: UiRect::all(px(10.0)),
        ..default()
    };
    commands
        .spawn((
            MeterWindow,
            window,
            BackgroundColor(Color::srgba(0.02, 0.02, 0.04, 0.62)),
            Visibility::Hidden,
            bevy::ui::FocusPolicy::Pass,
        ))
        .with_children(|root| {
            root.spawn(text(&f, i18n::t("Damage meter"), 15.0, GOLD));
            for field in [
                MeterField::Total,
                MeterField::Dps,
                MeterField::Peak,
                MeterField::Taken,
            ] {
                root.spawn((
                    field,
                    row(&f, i18n::t(field_label(field)), Color::WHITE),
                ));
            }
            root.spawn((
                Node {
                    height: px(1.0),
                    margin: UiRect::vertical(px(3.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.14)),
            ));
            for i in 0..5 {
                root.spawn((
                    MeterTopLine(i),
                    row(&f, "", WHITE),
                ));
            }
        });
}

/// 面板四行的标签文案。
fn field_label(field: MeterField) -> &'static str {
    match field {
        MeterField::Total => "Total damage",
        MeterField::Dps => "DPS",
        MeterField::Peak => "Peak DPS",
        MeterField::Taken => "Damage taken",
    }
}

fn row(
    font: &Handle<Font>,
    label: &str,
    color: Color,
) -> (Node, (Text, TextFont, TextColor)) {
    (
        Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            width: Val::Percent(100.0),
            ..default()
        },
        text(font, label, 12.0, color),
    )
}

fn toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut open: ResMut<MeterOpen>,
    phase: Res<State<Phase>>,
    mut windows: Query<&mut Visibility, With<MeterWindow>>,
) {
    if keys.just_pressed(KeyCode::KeyV) {
        open.0 = !open.0;
    }
    let want = open.0 && *phase.get() == Phase::Playing;
    for mut v in &mut windows {
        let target = if want {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        v.set_if_neq(target);
    }
}

/// 法术键 → 展示名:数据里的 DBC 名(zhCN 数据即中文),非能力来源用目录词条。
fn ability_label(key: &'static str, spells: Option<&crate::ui_action::Spells>) -> String {
    match key {
        "melee" => i18n::t("melee").to_string(),
        "thorns" => i18n::t("thorns").to_string(),
        _ => {
            let def = data::ability(key);
            hud::spell_name(spells, def.spell, def.key)
        }
    }
}

#[allow(clippy::type_complexity)]
fn refresh(
    time: Res<Time>,
    run: Res<Run>,
    spells: Option<Res<crate::ui_action::Spells>>,
    mut throttle: Local<f32>,
    mut fields: ParamSet<(
        Query<(&MeterField, &mut Text)>,
        Query<(&MeterTopLine, &mut Text)>,
    )>,
) {
    *throttle -= time.delta_secs();
    if *throttle > 0.0 {
        return;
    }
    *throttle = 0.25;
    let total = run.damage_done;    let mut tops = run.damage_by_ability.clone();
    tops.sort_by(|a, b| b.1.total_cmp(&a.1));
    let p = fields.p0();
    for (field, mut text) in p {
        let want = match field {
            MeterField::Total => hud::compact(total),
            MeterField::Dps => format!("{:.0}", run.dps_now),
            MeterField::Peak => format!("{:.0}", run.peak_dps),
            MeterField::Taken => hud::compact(run.damage_taken),
        };
        if **text != want {
            **text = want;
        }
    }
    let p = fields.p1();
    for (line, mut text) in p {
        let want = tops
            .get(line.0)
            .map(|(key, v)| {
                let share = if total > 0.0 { *v / total * 100.0 } else { 0.0 };
                let name = ability_label(key, spells.as_deref());
                format!("{name}  {} ({share:.0}%)", hud::compact(*v))
            })
            .unwrap_or_default();
        if **text != want {
            **text = want;
        }
    }
}
