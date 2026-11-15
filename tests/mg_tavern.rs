//! T703: the TAVERN brawl. Ruffians come through the doors in three
//! waves; fists, grips and kicked barrels are the whole answer. A punch
//! staggers, a punch on a stagger grabs, a grip hurled into the wall is a
//! KO - and the barmaid crossing with the ales is out of bounds, every
//! time. Fifteen down with a clean floor is the 92 the gold line demands.

use defender_of_the_crown::engine::gfx::Framebuffer;
use defender_of_the_crown::engine::input::{Input, Raw, KEY_LEFT, KEY_RIGHT, KEY_SPACE};
use defender_of_the_crown::minigames::framework::{MgState, Minigame, Tier};
use defender_of_the_crown::minigames::tavern::Tavern;

const PRESENT: u32 = 90;
const MARCH: u8 = 0;
const STAG: u8 = 1;
const DOWN: u8 = 4;
const ROGUES: usize = 15;

struct Sim {
    inp: Input,
    fb: Framebuffer,
    frame: u64,
}

impl Sim {
    fn new() -> Self {
        Self {
            inp: Input::new(),
            fb: Framebuffer::new(),
            frame: 0,
        }
    }
    fn frame(&mut self, g: &mut Tavern, events: &[Raw]) {
        for e in events {
            self.inp.push(*e);
        }
        self.frame += 1;
        self.inp.begin_frame(self.frame);
        g.tick(1.0 / 60.0, &self.inp, &mut self.fb);
    }
    fn idle(&mut self, g: &mut Tavern, ticks: u32) {
        for _ in 0..ticks {
            self.frame(g, &[]);
        }
    }
    fn hold(&mut self, g: &mut Tavern, code: u32) {
        self.frame(g, &[Raw::Key { code, pressed: true }]);
    }
    fn stop(&mut self, g: &mut Tavern) {
        self.frame(
            g,
            &[
                Raw::Key {
                    code: KEY_LEFT,
                    pressed: false,
                },
                Raw::Key {
                    code: KEY_RIGHT,
                    pressed: false,
                },
            ],
        );
    }
    fn punch(&mut self, g: &mut Tavern) {
        self.frame(
            g,
            &[
                Raw::Key {
                    code: KEY_SPACE,
                    pressed: true,
                },
                Raw::Key {
                    code: KEY_SPACE,
                    pressed: false,
                },
            ],
        );
        self.idle(g, 15); // let the cooldown lapse
    }
    /// A punch thrown only when the barmaid is out of the way: off the
    /// floor, clear of the fist lane, and - with a rogue in grip - clear
    /// of the whole hurl corridor.
    fn tap(&mut self, g: &mut Tavern) {
        loop {
            let (mx, mf) = g.maid();
            let (px, face) = g.px();
            let safe = if !mf {
                true
            } else if g.held().is_some() {
                if face == 1 {
                    mx < px - 8.0
                } else {
                    mx > px + 8.0
                }
            } else if face == 1 {
                mx < px - 4.0 || mx > px + 20.0
            } else {
                mx > px + 4.0 || mx < px - 20.0
            };
            if safe {
                break;
            }
            self.idle(g, 1); // hold position and let her skirts pass
        }
        self.punch(g);
    }
}

fn open_run(seed: u64) -> (Sim, Tavern) {
    let mut sim = Sim::new();
    let mut g = Tavern::new(seed);
    sim.idle(&mut g, PRESENT + 1); // burn the banner, room starts moving
    (sim, g)
}

/// Walk until `tx` sits in the punch lane (4..16 px ahead of the face).
fn line_up(sim: &mut Sim, g: &mut Tavern, tx: f32) -> bool {
    for _ in 0..500 {
        if g.stunned() {
            sim.idle(g, 1);
            continue;
        }
        let (px, face) = g.px();
        let d = tx - px;
        if face == 1 && (4.0..=16.0).contains(&d) {
            sim.stop(g);
            return true;
        }
        if face == -1 && (-16.0..=-4.0).contains(&d) {
            sim.stop(g);
            return true;
        }
        sim.hold(g, if d > 0.0 { KEY_RIGHT } else { KEY_LEFT });
    }
    false
}

/// Wait for the first rogue to be in play.
fn wait_live(sim: &mut Sim, g: &mut Tavern) -> usize {
    for _ in 0..600 {
        if let Some(i) = g.live_rogue() {
            return i;
        }
        sim.idle(g, 1);
    }
    panic!("no rogue ever entered the room");
}

#[test]
fn banner_holds_the_room() {
    let mut sim = Sim::new();
    let mut g = Tavern::new(1);
    sim.idle(&mut g, PRESENT); // banner still up on its last tick
    assert_eq!(g.wave(), (1, 0), "wave 1 not started behind the banner");
    assert_eq!(g.live_rogue(), None, "no rogue while the banner shows");
    let (mut sim, mut g) = open_run(1);
    let i = wait_live(&mut sim, &mut g);
    assert_eq!(g.rogue(i).1, MARCH, "the first rogue walks in");
}

#[test]
fn rogues_march_in_and_swing() {
    let (mut sim, mut g) = open_run(5);
    let i = wait_live(&mut sim, &mut g);
    // stand still and eat the brawl: a rogue closes to melee and swings
    for _ in 0..2000 {
        if g.stunned() {
            assert!(g.rogue(i).0 > 0.0, "a rogue in melee is on the floor");
            assert_eq!(g.ko(), 0, "an idle brawler knocks nobody down");
            return;
        }
        sim.idle(&mut g, 1);
    }
    panic!("no ruffian ever swung");
}

#[test]
fn punch_staggers_grabs_hurls() {
    let (mut sim, mut g) = open_run(9);
    let t = wait_live(&mut sim, &mut g);
    // one: the march staggers
    for _ in 0..20 {
        let (tx, ts) = g.rogue(t);
        if ts != MARCH {
            break;
        }
        if line_up(&mut sim, &mut g, tx) {
            sim.tap(&mut g);
        }
    }
    assert_eq!(g.rogue(t).1, STAG, "a punch staggers a marching rogue");
    // two: the stagger is grabbed
    for _ in 0..20 {
        if g.held().is_some() {
            break;
        }
        let (tx, _) = g.rogue(t);
        if line_up(&mut sim, &mut g, tx) {
            sim.tap(&mut g);
        }
    }
    assert_eq!(g.held(), Some(t), "a punch on a stagger grabs");
    // three: hurl him at a wall
    let (px, _) = g.px();
    sim.hold(
        &mut g,
        if px < 160.0 { KEY_LEFT } else { KEY_RIGHT },
    );
    sim.stop(&mut g);
    sim.tap(&mut g);
    assert_eq!(g.held(), None, "the grip is gone once hurled");
    for _ in 0..200 {
        if g.rogue(t).1 == DOWN {
            break;
        }
        sim.idle(&mut g, 1);
    }
    assert_eq!(g.rogue(t).1, DOWN, "the wall ends the flight");
    assert!(g.ko() >= 1, "the wall ends him at least");
    assert_eq!(g.score(), 6 * g.ko(), "six for every downed rogue");
}

#[test]
fn stun_drops_the_grip() {
    let (mut sim, mut g) = open_run(3);
    let t = wait_live(&mut sim, &mut g);
    for _ in 0..40 {
        if g.held() == Some(t) {
            break;
        }
        let (tx, ts) = g.rogue(t);
        if (ts == MARCH || ts == STAG) && line_up(&mut sim, &mut g, tx) {
            sim.tap(&mut g);
        }
    }
    assert_eq!(g.held(), Some(t), "gripped");
    // a friend's fist empties the arms
    for _ in 0..1200 {
        if g.held().is_none() {
            assert!(g.stunned(), "the grip is lost to a blow");
            assert_eq!(g.rogue(t).1, STAG, "the gripped rogue drops to a stagger");
            return;
        }
        sim.idle(&mut g, 1);
    }
    panic!("never got hit while holding a rogue");
}

#[test]
fn barrel_flattens_the_line() {
    let (mut sim, mut g) = open_run(11);
    let t = wait_live(&mut sim, &mut g); // through the left door
    // stand right of the middle barrel (x = 200) and kick it left into the line
    for _ in 0..120 {
        let (px, _) = g.px();
        if px > 206.0 {
            break;
        }
        sim.hold(&mut g, KEY_RIGHT);
    }
    sim.hold(&mut g, KEY_LEFT); // face left, one step off the barrel
    sim.stop(&mut g);
    assert_eq!(g.barrels_left(), 3);
    sim.punch(&mut g);
    assert_eq!(g.barrels_left(), 2, "the barrel is away");
    for _ in 0..800 {
        if g.ko() == 1 {
            break;
        }
        sim.idle(&mut g, 1);
    }
    assert_eq!(g.rogue(t).1, DOWN, "the barrel flattened the marcher");
    assert_eq!(g.ko(), 1);
    assert_eq!(g.fouls(), 0, "the floor was clean for the roll");
    assert_eq!(g.score(), 6);
}

#[test]
fn the_barmaid_is_out_of_bounds() {
    let (mut sim, mut g) = open_run(6);
    for _ in 0..4000 {
        let (px, face) = g.px();
        let (mx, mf) = g.maid();
        if mf {
            let d = mx - px;
            if d.abs() > 4.0 && d.abs() < 15.0 && face == d.signum() as i32 {
                sim.stop(&mut g);
                sim.punch(&mut g);
                if g.fouls() > 0 {
                    break;
                }
            } else {
                sim.hold(&mut g, if d > 0.0 { KEY_RIGHT } else { KEY_LEFT });
            }
        } else {
            sim.stop(&mut g);
        }
    }
    assert_eq!(g.fouls(), 1, "a fist in her lane is a foul");
    assert!(!g.maid().1, "she flees the room");
    assert_eq!(g.score(), 0, "the howl costs every point so far");
}

#[test]
fn clean_sweep_earns_the_gold() {
    let (mut sim, mut g) = open_run(42);
    let mut guard = 0u32;
    while g.ko() < ROGUES as u32 {
        guard += 1;
        assert!(guard < 60_000, "sweep stalled at ko {}", g.ko());
        if g.stunned() {
            sim.idle(&mut g, 1);
            continue;
        }
        if let Some(h) = g.held() {
            // hurl at the near wall - short corridor the maid can skirt
            let (px, _) = g.px();
            let (mx, mf) = g.maid();
            if mf && (mx - px).abs() < 70.0 {
                sim.idle(&mut g, 1);
                continue;
            }
            sim.hold(&mut g, if px < 160.0 { KEY_LEFT } else { KEY_RIGHT });
            sim.stop(&mut g);
            sim.tap(&mut g);
            for _ in 0..120 {
                if g.rogue(h).1 == DOWN {
                    break;
                }
                sim.idle(&mut g, 1);
            }
            assert_eq!(g.rogue(h).1, DOWN, "a hurl must end on the floor");
            continue;
        }
        let (px, _) = g.px();
        let t = (0..ROGUES)
            .filter(|&i| {
                let (x, s) = g.rogue(i);
                x > 0.0 && (s == MARCH || s == STAG)
            })
            .min_by(|&a, &b| {
                let da = (g.rogue(a).0 - px).abs();
                let db = (g.rogue(b).0 - px).abs();
                da.total_cmp(&db)
            });
        let Some(t) = t else {
            sim.idle(&mut g, 1);
            continue;
        };
        let (tx, _) = g.rogue(t);
        if line_up(&mut sim, &mut g, tx) {
            sim.tap(&mut g);
        }
    }
    assert_eq!(g.fouls(), 0, "a clean sweep never fouled");
    assert_eq!(g.wave(), (3, 6), "all three waves were fought");
    assert_eq!(g.score(), 92, "fifteen down plus the clean-floor +2");
    assert!(matches!(g.state(), MgState::Won(Tier::Gold)));
}

#[test]
fn the_seed_alone_picks_her_return() {
    // two seeds, two barmaid return sides; nothing else differs at t=0
    let a = Tavern::new(1);
    let b = Tavern::new(2);
    assert_eq!(a.px(), b.px(), "the room itself is seedless");
    assert_eq!(a.maid(), b.maid());
    let (mut sim, mut g) = open_run(1);
    // foul her once and watch where she comes back: the room is alive
    for _ in 0..4000 {
        let (px, face) = g.px();
        let (mx, mf) = g.maid();
        if mf {
            let d = mx - px;
            if d.abs() > 4.0 && d.abs() < 15.0 && face == d.signum() as i32 {
                sim.stop(&mut g);
                sim.punch(&mut g);
                if g.fouls() > 0 {
                    break;
                }
            } else {
                sim.hold(&mut g, if d > 0.0 { KEY_RIGHT } else { KEY_LEFT });
            }
        } else {
            sim.stop(&mut g);
        }
    }
    assert_eq!(g.fouls(), 1);
    sim.idle(&mut g, 101); // the flight timer lapses and she re-enters
    let (mx, _) = g.maid();
    assert!(
        (mx - 60.0).abs() < 14.0 || (mx - 220.0).abs() < 14.0,
        "she comes back at a counter end, not center stage (x={mx})"
    );
}
