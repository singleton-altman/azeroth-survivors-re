//! Touch controls: the left thumb walks, the right thumb aims, and three buttons cover what the
//! keyboard does in play (`F`, Space, Esc).
//!
//! Everything here is additive. With no touch the [resource](Touch) stays empty and the keyboard
//! and mouse paths behave exactly as before; the on-screen buttons stay hidden until a finger
//! lands (or the page says its pointer is coarse, `?touch=1`).
//!
//! The buttons are hit-tested against [`Touches`] here rather than through `Interaction`, because
//! `bevy_ui`'s focus system hit-tests the *first* pressed finger only: while the thumb holds the
//! stick, a second finger on a button would be tested at the stick's position and never land.

use bevy::input::touch::Touches;
use bevy::prelude::*;
use bevy::ui::{FocusPolicy, UiScale};
use bevy::window::PrimaryWindow;

use benilla_world::view::WorldCamera;
use benilla_world::world_point::WorldPoint;

use super::hero::{ground_under, AimPoint, CamMode, CamRig, Hero};
use super::hud::{IconDone, SpellIcon};
use super::{data, i18n, Phase, Run, Selection};

/// Full deflection, logical px from where the thumb landed.
const STICK_RADIUS: f32 = 64.0;
/// Below this fraction of [`STICK_RADIUS`] the hero stands still, so a resting thumb is no drift.
const STICK_DEADZONE: f32 = 0.14;
/// How long a tap's aim outranks the mouse — there is no mouse on a phone, and the hero faces the
/// last tap until it walks.
const AIM_HOLD: f32 = 1.2;

/// Button sizes and insets in logical pixels, which is what a thumb measures. `UiScale`
/// multiplies node px by the window height over 768, so [`layout`] divides by it: a button sized
/// for a 1080p window would otherwise come out half-size on a phone.
const CAST_SIZE: f32 = 84.0;
const JUMP_SIZE: f32 = 62.0;
const PAUSE_SIZE: f32 = 46.0;
/// Insets are measured from the corner named beside them.
const CAST_INSET: Vec2 = Vec2::new(20.0, 100.0); // from the bottom right
const JUMP_INSET: Vec2 = Vec2::new(116.0, 22.0);
/// Top left: the top right is the minimap's cluster, which must stay tappable.
const PAUSE_INSET: Vec2 = Vec2::new(14.0, 14.0); // from the top left
/// Under the pause button: the view lock.
const CAMERA_INSET: Vec2 = Vec2::new(14.0, 70.0); // from the top left

const STICK_BASE: f32 = 128.0;
const STICK_KNOB: f32 = 54.0;

/// The menu bar's height and its padding, logical px.
const BAR_H: f32 = 84.0;
const BAR_PAD: f32 = 10.0;

const BUTTON_FACE: Color = Color::srgba(0.055, 0.06, 0.075, 0.62);
const BUTTON_FACE_OFF: Color = Color::srgba(0.055, 0.06, 0.075, 0.34);
const BUTTON_EDGE: Color = Color::srgba(0.95, 0.78, 0.24, 0.55);
const BUTTON_EDGE_OFF: Color = Color::srgba(0.55, 0.55, 0.55, 0.35);
const GOLD: Color = Color::srgba(0.95, 0.78, 0.24, 0.9);

/// What this frame's fingers mean. Written by [`read`] and [`presses`], spent by the hero and the
/// mobility spell.
#[derive(Resource, Default)]
pub(super) struct Touch {
    /// The stick's world-space direction (unit length), zero when it is not pushed.
    pub(super) move_dir: Vec3,
    /// One-frame edges; [`read`] clears them at the top of the frame that follows.
    pub(super) jump: bool,
    pub(super) cast: bool,
    /// Seconds left for which a touch-set [`AimPoint`] outranks the mouse.
    pub(super) aim_hold: f32,
    /// A finger has been seen, or the page said the pointer is coarse: the buttons show.
    pub(super) active: bool,
    /// Where the walking finger went down and how far it is pushed (clamped), in logical px.
    pub(super) stick_at: Vec2,
    pub(super) stick_push: Vec2,
    /// The finger on the stick.
    stick: Option<(u64, Vec2)>,
    /// The finger that last aimed.
    aim: Option<u64>,
    /// The centroid of a two-finger drag, while one is in progress.
    orbit_at: Option<Vec2>,
}

impl Touch {
    pub(super) fn holding(&self) -> bool {
        self.stick.is_some()
    }
}

/// The on-screen buttons, by what they do.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum MobileButton {
    /// The class's movement spell, from level 5 (the mobility HUD slot's touch twin).
    Cast,
    Jump,
    Pause,
    /// Lock / unlock the 2.5D view: `2.5D` locked, `FREE` to swing and tilt with two fingers.
    Camera,
}

#[derive(Component)]
struct MobileRoot;
#[derive(Component)]
struct StickBase;
#[derive(Component)]
struct StickKnob;
#[derive(Component)]
struct CastIcon;
#[derive(Component)]
struct CastCooldown;

/// A phone's way in: the glue pick screen is drawn at 1024×768 and comes out half-size in a
/// phone's landscape window. Touch gets this bar instead — tap a side to cycle it, tap START.
#[derive(Resource, Default)]
struct Pick {
    class: usize,
    map: usize,
}

#[derive(Component)]
struct MobileMenu;
#[derive(Component)]
struct PickBar;
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum PickButton {
    Class,
    Map,
    Start,
}
/// Marks the label inside a [`PickButton`]; the label carries the same button kind so one query
/// can fill it.
#[derive(Component)]
struct PickText;

/// The view button's own label (`2.5D` / `FREE`).
#[derive(Component)]
struct ViewLabel;

pub(super) fn plugin(app: &mut App) {
    // iOS is always a touch screen; a page says so for itself (`?touch=1`, set from `(pointer:
    // coarse)`); anywhere else the controls wait for a finger to land.
    let active = cfg!(target_os = "ios")
        || crate::webenv::var("WOW_TOUCH").is_some_and(|v| v != "0");
    app.insert_resource(Touch {
        active,
        ..default()
    })
    .init_resource::<Pick>()
    .add_systems(Startup, (setup, setup_menu, super::hero::restore_view))
    // Ahead of the systems that spend it, and in this order: the fingers, the aim, then the
    // buttons (which set this frame's edges), then the mobility cast and the walk. `aim_cursor`,
    // `cast_mobility` and `drive_hero` live in `WorldStage::Input`'s chain (`mod.rs`); these
    // constraints place the two systems below against them.
    .add_systems(Update, read.before(super::hero::aim_cursor))
    .add_systems(
        Update,
        presses
            .after(read)
            .before(super::mobility::cast_mobility)
            .before(super::hero::drive_hero),
    )
    .add_systems(
        Update,
        (layout, draw_stick, cast_button, view_button, pick, refresh_pick)
            .chain()
            .after(super::hud::apply_ui_scale),
    );
}

/// The stick and the three buttons, all hidden until a finger shows up.
fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    let font: Handle<Font> = assets.load("mpq://Fonts/ARIALN.ttf");
    let circle = |size: f32| Node {
        position_type: PositionType::Absolute,
        width: Val::Px(size),
        height: Val::Px(size),
        border_radius: BorderRadius::MAX,
        ..default()
    };
    let label = |text: &str| {
        (
            Text::new(text),
            TextFont {
                font: font.clone(),
                font_size: 13.0,
                ..default()
            },
            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.92)),
            TextLayout::new(Justify::Center, LineBreak::NoWrap),
        )
    };
    let centered = || Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    };

    commands
        .spawn((
            MobileRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            // Above the in-run HUD (400), below the loading cover (1000). The pause screen is
            // the HUD's own, so these hide while it is up (`layout`).
            GlobalZIndex(850),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((
                StickBase,
                circle(STICK_BASE),
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.07)),
                BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.22)),
                Visibility::Hidden,
            ))
            .with_children(|base| {
                base.spawn((
                    StickKnob,
                    circle(STICK_KNOB),
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.28)),
                ));
            });

            // Pause, out of both thumbs' way in the top right.
            root.spawn((
                MobileButton::Pause,
                circle(PAUSE_SIZE),
                BackgroundColor(BUTTON_FACE),
                BorderColor::all(BUTTON_EDGE),
                FocusPolicy::Block,
            ))
            .with_children(|b| {
                b.spawn((centered(), FocusPolicy::Pass))
                    .with_children(|c| {
                        c.spawn(label("II"));
                    });
            });

            // The view lock, under the pause button: `2.5D` when locked, `FREE` when a two-finger
            // drag may swing the camera.
            root.spawn((
                MobileButton::Camera,
                circle(PAUSE_SIZE),
                BackgroundColor(BUTTON_FACE),
                BorderColor::all(BUTTON_EDGE),
                FocusPolicy::Block,
            ))
            .with_children(|b| {
                b.spawn((centered(), FocusPolicy::Pass))
                    .with_children(|c| {
                        c.spawn((ViewLabel, label("2.5D")));
                    });
            });

            root.spawn((
                MobileButton::Jump,
                circle(JUMP_SIZE),
                BackgroundColor(BUTTON_FACE),
                BorderColor::all(BUTTON_EDGE),
                FocusPolicy::Block,
            ))
            .with_children(|b| {
                b.spawn((centered(), FocusPolicy::Pass))
                    .with_children(|c| {
                        c.spawn(label("JUMP"));
                    });
            });

            // The cast: the mobility spell's own icon, its cooldown filling from the bottom.
            root.spawn((
                MobileButton::Cast,
                circle(CAST_SIZE),
                BackgroundColor(BUTTON_FACE_OFF),
                BorderColor::all(BUTTON_EDGE_OFF),
                FocusPolicy::Block,
            ))
            .with_children(|b| {
                b.spawn((
                    CastIcon,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    ImageNode {
                        color: Color::srgba(0.0, 0.0, 0.0, 0.6),
                        ..default()
                    },
                    SpellIcon(0),
                    FocusPolicy::Pass,
                ));
                b.spawn((
                    CastCooldown,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        bottom: Val::Px(0.0),
                        width: Val::Percent(100.0),
                        height: Val::Percent(0.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.62)),
                    FocusPolicy::Pass,
                ));
            });
        });
}

/// The fingers: the left half walks, the right half aims, and two fingers swing an unlocked
/// camera. A lift clears all three, and so does a lost event — a stuck direction would walk the
/// hero off the map.
#[allow(clippy::too_many_arguments)]
fn read(
    touches: Res<Touches>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cams: Query<(&Camera, &Transform), With<WorldCamera>>,
    hero: Query<&Transform, (With<Hero>, Without<WorldCamera>)>,
    world: WorldPoint,
    time: Res<Time>,
    phase: Res<State<Phase>>,
    mut rig: ResMut<CamRig>,
    mut touch: ResMut<Touch>,
    mut aim: ResMut<AimPoint>,
) {
    touch.jump = false;
    touch.cast = false;
    touch.aim_hold = (touch.aim_hold - time.delta_secs()).max(0.0);
    let Ok(window) = windows.single() else {
        return;
    };
    let (w, h) = (window.width(), window.height());

    let playing = *phase.get() == Phase::Playing;

    // Two fingers anywhere are a camera gesture, not a walk: swing and tilt an unlocked view by
    // their centroid's travel (a locked one ignores the delta and stays where it is).
    let (mut fingers, mut at) = (0usize, Vec2::ZERO);
    for t in touches.iter() {
        fingers += 1;
        at += t.position();
    }
    if playing && fingers >= 2 {
        at /= fingers as f32;
        if let Some(prev) = touch.orbit_at {
            rig.orbit_by((at - prev) * 0.6);
        }
        touch.orbit_at = Some(at);
        touch.stick = None;
        touch.move_dir = Vec3::ZERO;
        touch.stick_push = Vec2::ZERO;
        touch.aim = None;
        return;
    }
    touch.orbit_at = None;

    for t in touches.iter_just_pressed() {
        touch.active = true;
        // The menu's bar and the pause screen take their own taps; only play steers a hero.
        if !playing {
            continue;
        }
        let p = t.position();
        if over_button(p, w, h).is_some() {
            continue; // `presses` owns this one
        }
        if p.x < w * 0.5 {
            if touch.stick.is_none() {
                touch.stick = Some((t.id(), p));
                touch.stick_at = p;
                touch.stick_push = Vec2::ZERO;
            }
        } else if touch.aim.is_none() {
            touch.aim = Some(t.id());
            aim_at(p, &cams, &hero, &world, &mut touch, &mut aim);
        }
    }

    for t in touches.iter() {
        let p = t.position();
        if let Some((id, origin)) = touch.stick {
            if id == t.id() {
                let d = p - origin;
                let len = d.length();
                touch.stick_push = if len > STICK_RADIUS {
                    d * (STICK_RADIUS / len)
                } else {
                    d
                };
                let v = touch.stick_push / STICK_RADIUS;
                // Screen x is world x and screen down is world +z: the locked camera looks down
                // -z, so pushing up walks away from it.
                touch.move_dir = if v.length() < STICK_DEADZONE {
                    Vec3::ZERO
                } else {
                    Vec3::new(v.x, 0.0, v.y).normalize()
                };
            }
        }
        if touch.aim == Some(t.id()) {
            aim_at(p, &cams, &hero, &world, &mut touch, &mut aim);
        }
    }

    if let Some((id, _)) = touch.stick {
        if touches.get_pressed(id).is_none() {
            touch.stick = None;
            touch.move_dir = Vec3::ZERO;
            touch.stick_push = Vec2::ZERO;
        }
    }
    if touch.aim.is_some_and(|id| touches.get_pressed(id).is_none()) {
        touch.aim = None;
    }
}

/// The terrain under a screen position, aimed the way the mouse aims.
fn aim_at(
    p: Vec2,
    cams: &Query<(&Camera, &Transform), With<WorldCamera>>,
    hero: &Query<&Transform, (With<Hero>, Without<WorldCamera>)>,
    world: &WorldPoint,
    touch: &mut Touch,
    aim: &mut AimPoint,
) {
    let (Ok((cam, tf)), Ok(h)) = (cams.single(), hero.single()) else {
        return;
    };
    if let Some(at) = ground_under(cam, tf, p, h.translation.y, world) {
        aim.0 = Some(at);
        touch.aim_hold = AIM_HOLD;
    }
}

/// The buttons' presses: an edge on `just_pressed`, and the pause toggle straight to the state.
fn presses(
    touches: Res<Touches>,
    windows: Query<&Window, With<PrimaryWindow>>,
    phase: Res<State<Phase>>,
    mut rig: ResMut<CamRig>,
    mut touch: ResMut<Touch>,
    mut next: ResMut<NextState<Phase>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let (w, h) = (window.width(), window.height());
    for t in touches.iter_just_pressed() {
        touch.active = true;
        match over_button(t.position(), w, h) {
            Some(MobileButton::Cast) => touch.cast = true,
            Some(MobileButton::Jump) => touch.jump = true,
            Some(MobileButton::Camera) => {
                rig.toggle_lock();
                super::hero::save_view(&rig);
            }
            Some(MobileButton::Pause) => match phase.get() {
                Phase::Playing => next.set(Phase::Paused),
                Phase::Paused => next.set(Phase::Playing),
                _ => {}
            },
            None => {}
        }
    }
}

/// Which corner a button hangs off.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Corner {
    BottomRight,
    TopLeft,
}

/// A button's size, its inset from [`Corner`], and that corner.
fn place(btn: MobileButton) -> (f32, Vec2, Corner) {
    match btn {
        MobileButton::Cast => (CAST_SIZE, CAST_INSET, Corner::BottomRight),
        MobileButton::Jump => (JUMP_SIZE, JUMP_INSET, Corner::BottomRight),
        MobileButton::Pause => (PAUSE_SIZE, PAUSE_INSET, Corner::TopLeft),
        MobileButton::Camera => (PAUSE_SIZE, CAMERA_INSET, Corner::TopLeft),
    }
}

/// Whether a logical-pixel point is inside one of the buttons. The rects are the ones [`layout`]
/// gives the nodes, so a tap lands where it looks.
fn over_button(p: Vec2, w: f32, h: f32) -> Option<MobileButton> {
    const ALL: [MobileButton; 4] = [
        MobileButton::Cast,
        MobileButton::Jump,
        MobileButton::Pause,
        MobileButton::Camera,
    ];
    ALL.into_iter().find(|&btn| {
        let (size, inset, corner) = place(btn);
        let (left, top) = match corner {
            Corner::BottomRight => (w - inset.x - size, h - inset.y - size),
            Corner::TopLeft => (inset.x, inset.y),
        };
        p.x >= left && p.x <= left + size && p.y >= top && p.y <= top + size
    })
}

/// Place and size the buttons, and show them while there is something to do. Recomputed every
/// frame because a phone rotates, and the scale follows the window height.
fn layout(
    phase: Res<State<Phase>>,
    touch: Res<Touch>,
    scale: Res<UiScale>,
    mut root: Query<&mut Visibility, With<MobileRoot>>,
    mut buttons: Query<(&MobileButton, &mut Node), Without<PickBar>>,
    mut bar: Query<&mut Node, (With<PickBar>, Without<MobileButton>)>,
    mut fonts: Query<&mut TextFont, With<PickText>>,
) {
    let k = 1.0 / scale.0.max(0.05);
    for (btn, mut node) in &mut buttons {
        let (size, inset, corner) = place(*btn);
        let px = Val::Px(size * k);
        if node.width != px {
            node.width = px;
            node.height = px;
        }
        let along = Val::Px(inset.x * k);
        let across = Val::Px(inset.y * k);
        match corner {
            Corner::BottomRight => {
                if node.right != along || node.bottom != across {
                    node.right = along;
                    node.bottom = across;
                    node.left = Val::Auto;
                    node.top = Val::Auto;
                }
            }
            Corner::TopLeft => {
                if node.left != along || node.top != across {
                    node.left = along;
                    node.top = across;
                    node.right = Val::Auto;
                    node.bottom = Val::Auto;
                }
            }
        }
    }
    // The menu bar is laid out a step above the home indicator's reach.
    if let Ok(mut node) = bar.single_mut() {
        let height = Val::Px((BAR_H + BAR_PAD) * k);
        if node.height != height {
            node.height = height;
            node.padding = UiRect::all(Val::Px(BAR_PAD * k));
            node.column_gap = Val::Px(BAR_PAD * k);
        }
    }
    for mut font in &mut fonts {
        let size = 15.0 * k;
        if font.font_size != size {
            font.font_size = size;
        }
    }
    // Only in play: paused is the pause screen's own dialogs (their Resume is a tap), and the
    // level-up cards own the screen while they are up. Every one of these is above the in-run HUD
    // and below the loading cover.
    let show = touch.active && *phase.get() == Phase::Playing;
    if let Ok(mut vis) = root.single_mut() {
        let want = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
}

/// The stick under the thumb that holds it: its base where the finger went down, its knob pushed
/// out by the deflection.
fn draw_stick(
    touch: Res<Touch>,
    phase: Res<State<Phase>>,
    scale: Res<UiScale>,
    mut base: Query<(&mut Node, &mut Visibility), (With<StickBase>, Without<StickKnob>)>,
    mut knob: Query<&mut Node, (With<StickKnob>, Without<StickBase>)>,
) {
    let show = touch.holding() && *phase.get() == Phase::Playing;
    let k = 1.0 / scale.0.max(0.05);
    if let Ok((mut node, mut vis)) = base.single_mut() {
        let want = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
        if show {
            let left = Val::Px((touch.stick_at.x - STICK_BASE / 2.0) * k);
            let top = Val::Px((touch.stick_at.y - STICK_BASE / 2.0) * k);
            if node.left != left {
                node.left = left;
                node.top = top;
            }
        }
    }
    if let Ok(mut node) = knob.single_mut() {
        let seat = (STICK_BASE - STICK_KNOB) / 2.0;
        let left = Val::Px((seat + touch.stick_push.x) * k);
        let top = Val::Px((seat + touch.stick_push.y) * k);
        if node.left != left {
            node.left = left;
            node.top = top;
        }
    }
}

/// The cast button's face: the class's spell icon once learned, dimmed until then, with the
/// cooldown filling in from the bottom.
fn cast_button(
    run: Res<Run>,
    mut commands: Commands,
    mut icon: Query<(Entity, Option<&SpellIcon>, &mut Visibility), With<CastIcon>>,
    mut button: Query<(&MobileButton, &mut BackgroundColor, &mut BorderColor)>,
    mut cd: Query<&mut Node, With<CastCooldown>>,
) {
    let def = run.mobility();
    let (face, line) = if def.is_some() {
        (BUTTON_FACE, BUTTON_EDGE)
    } else {
        (BUTTON_FACE_OFF, BUTTON_EDGE_OFF)
    };
    for (btn, mut bg, mut edge) in &mut button {
        if *btn != MobileButton::Cast {
            continue;
        }
        if bg.0 != face {
            bg.0 = face;
        }
        if edge.top != line {
            *edge = BorderColor::all(line);
        }
    }
    // Before level 5 there is nothing to cast: an empty dial, not a question mark.
    let Some(def) = def else {
        for (_, _, mut vis) in &mut icon {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
        }
        return;
    };
    // A new spell re-arms the icon fill ([`super::hud::fill_icons`]).
    if let Ok((e, have, mut vis)) = icon.single_mut() {
        if *vis != Visibility::Inherited {
            *vis = Visibility::Inherited;
        }
        if have.is_none_or(|s| s.0 != def.spell) {
            commands
                .entity(e)
                .remove::<IconDone>()
                .insert(SpellIcon(def.spell));
        }
    }
    if let Ok(mut node) = cd.single_mut() {
        let left = (run.mobile.cd / def.cooldown.max(0.01)).clamp(0.0, 1.0);
        let height = Val::Percent(left * 100.0);
        if node.height != height {
            node.height = height;
        }
    }
}

/// The view button's face: `2.5D` lit while the camera is locked, `FREE` dimmed while it may be
/// dragged. The lock itself lives on [`CamRig`]; this only shows what it is.
fn view_button(
    rig: Res<CamRig>,
    mut button: Query<(&MobileButton, &mut BackgroundColor, &mut BorderColor)>,
    mut label: Query<(&mut Text, &mut TextColor), With<ViewLabel>>,
) {
    let locked = rig.mode == CamMode::Fixed;
    for (btn, mut bg, mut edge) in &mut button {
        if *btn != MobileButton::Camera {
            continue;
        }
        let (face, line) = if locked {
            (BUTTON_FACE, BUTTON_EDGE)
        } else {
            (BUTTON_FACE_OFF, BUTTON_EDGE_OFF)
        };
        if bg.0 != face {
            bg.0 = face;
        }
        if edge.top != line {
            *edge = BorderColor::all(line);
        }
    }
    for (mut text, mut color) in &mut label {
        let want = if locked { "2.5D" } else { "FREE" };
        if **text != want {
            **text = want.to_string();
        }
        let ink = if locked {
            GOLD
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.9)
        };
        if color.0 != ink {
            color.0 = ink;
        }
    }
}

/// The menu bar: a full-width strip along the bottom, with the class, the battleground and START.
/// It covers the glue screen's own Enter World button, so a thumb has one place to go.
fn setup_menu(mut commands: Commands, assets: Res<AssetServer>) {
    let font: Handle<Font> = assets.load("mpq://Fonts/ARIALN.ttf");
    commands
        .spawn((
            MobileMenu,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            // Stops the focus pass: the glue screen under this stays untappable while it is up.
            FocusPolicy::Block,
            GlobalZIndex(860),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((
                PickBar,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    height: Val::Px(BAR_H + BAR_PAD),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Stretch,
                    padding: UiRect::all(Val::Px(BAR_PAD)),
                    column_gap: Val::Px(BAR_PAD),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.02, 0.02, 0.03, 0.86)),
                FocusPolicy::Pass,
            ))
            .with_children(|bar| {
                for (kind, grow) in [
                    (PickButton::Class, 1.35_f32),
                    (PickButton::Map, 1.6),
                    (PickButton::Start, 0.9),
                ] {
                    bar.spawn((
                        kind,
                        Button,
                        Node {
                            flex_grow: grow,
                            height: Val::Percent(100.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            border_radius: BorderRadius::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(BUTTON_FACE),
                        BorderColor::all(BUTTON_EDGE),
                        FocusPolicy::Block,
                    ))
                    .with_children(|b| {
                        b.spawn((
                            kind,
                            PickText,
                            Text::new(""),
                            TextFont {
                                font: font.clone(),
                                font_size: 15.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TextLayout::new(Justify::Center, LineBreak::NoWrap),
                            FocusPolicy::Pass,
                        ));
                    });
                }
            });
        });
}

/// A tap on the bar: cycle the class or the battleground, or start the run.
fn pick(
    mut presses: Query<(&PickButton, &Interaction), (Changed<Interaction>, Without<PickText>)>,
    phase: Res<State<Phase>>,
    creatures: Option<Res<crate::entities::Creatures>>,
    mut pick: ResMut<Pick>,
    mut selection: ResMut<Selection>,
    mut run: ResMut<Run>,
    mut next: ResMut<NextState<Phase>>,
) {
    if *phase.get() != Phase::Menu {
        return;
    }
    for (btn, interaction) in &mut presses {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match btn {
            PickButton::Class => pick.class = (pick.class + 1) % data::CLASSES.len(),
            PickButton::Map => pick.map = (pick.map + 1) % data::MAPS.len(),
            PickButton::Start => {
                let Some(creatures) = creatures.as_deref() else {
                    continue; // the looks resolve from the creature data; it is never far off
                };
                let class = pick.class % data::CLASSES.len();
                let looks = super::hud::looks_for(&data::CLASSES[class], creatures);
                if looks.is_empty() {
                    continue;
                }
                selection.class = Some(class);
                selection.looks = looks;
                selection.look = Some(0);
                selection.map = pick.map % data::MAPS.len();
                selection.version += 1;
                if super::hud::start_run(&mut run, &mut selection) {
                    next.set(Phase::Playing);
                }
            }
        }
    }
}

/// The bar's labels and colours, and whether it shows at all: only at the menu, and only once a
/// finger (or the page's coarse-pointer hint) has said this is a touch screen.
fn refresh_pick(
    pick: Res<Pick>,
    phase: Res<State<Phase>>,
    touch: Res<Touch>,
    mut menu: Query<&mut Visibility, With<MobileMenu>>,
    mut faces: Query<
        (&PickButton, &mut BackgroundColor, &mut BorderColor),
        (Without<PickText>, Without<MobileMenu>),
    >,
    mut texts: Query<(&PickButton, &mut Text), With<PickText>>,
) {
    let class = &data::CLASSES[pick.class % data::CLASSES.len()];
    let map = &data::MAPS[pick.map % data::MAPS.len()];
    let tint = Color::srgba(
        f32::from((class.color >> 16) as u8) / 255.0,
        f32::from((class.color >> 8) as u8) / 255.0,
        f32::from(class.color as u8) / 255.0,
        0.42,
    );
    for (btn, mut bg, mut edge) in &mut faces {
        let (face, line) = match btn {
            PickButton::Class => (tint, BUTTON_EDGE),
            PickButton::Map => (BUTTON_FACE, BUTTON_EDGE),
            PickButton::Start => (Color::srgba(0.32, 0.24, 0.04, 0.85), GOLD),
        };
        if bg.0 != face {
            bg.0 = face;
        }
        if edge.top != line {
            *edge = BorderColor::all(line);
        }
    }
    for (btn, mut text) in &mut texts {
        let want = match btn {
            PickButton::Class => format!("{}  ▸", i18n::t(class.name)),
            PickButton::Map => format!("{}  ▸", i18n::t(map.name)),
            PickButton::Start => "START".to_string(),
        };
        if **text != want {
            **text = want;
        }
    }
    if let Ok(mut vis) = menu.single_mut() {
        let want = if touch.active && *phase.get() == Phase::Menu {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
}
