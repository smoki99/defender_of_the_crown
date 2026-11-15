//! T704: THE DRAGON'S HOARD. One try at the sleepin' dragon: creep past
//! the torchlit hall, pocket the sacks and the cursed crown, and reach the
//! west door before he wakes - or before the fire finds thy footprint.
//! The sleep cycle (sleep -> doze -> gaze -> settle) and the volley
//! (roar -> mark -> flame) are fully deterministic; every test below plans
//! its route around the seams `dragon()`, `volley()` and `loot()`.

use defender_of_the_crown::engine::gfx::{Framebuffer, BLACK};
use defender_of_the_crown::engine::input::{Input, Raw, KEY_DOWN, KEY_LEFT, KEY_RIGHT, KEY_UP};
use defender_of_the_crown::minigames::dragon::Dragon;
use defender_of_the_crown::minigames::framework::{MgState, Minigame, Tier};

const PRESENT: u32 = 90;
const CYCLE: u32 = 150;
// hall geometry (mirrors the module's private tables)
const X_MIN: f32 = 8.0;
const Y_MAX: f32 = 222.0;
const DOOR_ROW: f32 = 190.0; // any py in [150,216) is "at the door row"

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
    fn frame(&mut self, g: &mut Dragon, events: &[Raw]) {
        for e in events {
            self.inp.push(*e);
        }
        self.frame += 1;
        self.inp.begin_frame(self.frame);
        g.tick(1.0 / 60.0, &self.inp, &mut self.fb);
    }
    fn idle(&mut self, g: &mut Dragon, ticks: u32) {
        for _ in 0..ticks {
            self.frame(g, &[]);
        }
    }
    fn key(code: u32, held: bool) -> Raw {
        Raw::Key {
            code,
            pressed: held,
        }
    }
    /// One tick with an explicit four-axis level: `hx`/`vy` of -1, 0, 1.
    fn axes(&mut self, g: &mut Dragon, hx: i32, vy: i32) {
        self.frame(
            g,
            &[
                Self::key(KEY_LEFT, hx < 0),
                Self::key(KEY_RIGHT, hx > 0),
                Self::key(KEY_UP, vy < 0),
                Self::key(KEY_DOWN, vy > 0),
            ],
        );
    }
    fn stand_still(&mut self, g: &mut Dragon) {
        self.axes(g, 0, 0);
    }
    /// Diagonal walk to (`tx`, `ty`), stopping at the first frame inside
    /// 1.5 px on both axes; ends standing still. Loot grab radii (10/14)
    /// are what make passing within range of a sack pick it up.
    fn walk(&mut self, g: &mut Dragon, tx: f32, ty: f32) {
        for _ in 0..1500 {
            let (px, py, _) = g.pos();
            let dx = tx - px;
            let dy = ty - py;
            if dx.abs() < 1.5 && dy.abs() < 1.5 {
                self.stand_still(g);
                return;
            }
            self.axes(
                g,
                if dx.abs() < 1.5 {
                    0
                } else if dx > 0.0 {
                    1
                } else {
                    -1
                },
                if dy.abs() < 1.5 {
                    0
                } else if dy > 0.0 {
                    1
                } else {
                    -1
                },
            );
        }
        let (px, py, _) = g.pos();
        panic!(
            "walk never reached ({tx},{ty}): stuck at ({px:.1},{py:.1}) dragon={:?}",
            g.dragon()
        );
    }
    /// Walk to a cover pillar and verify the nook actually hides.
    fn hide_at(&mut self, g: &mut Dragon, cx: f32, cy: f32) {
        self.cycle0(g); // first leg must not cross a gaze sweep
        self.walk(g, cx, cy);
        assert!(g.hidden(), "({cx},{cy}) should hide the thief");
    }
    /// Wait for the FIRST tick of a fresh sleep window - the only safe
    /// launch pad for a leg across the open floor.
    fn cycle0(&mut self, g: &mut Dragon) {
        // wait out the tail of the sleep window first, then take its start
        for _ in 0..4 * CYCLE {
            if g.dragon().0 != 0 {
                break;
            }
            self.idle(g, 1);
        }
        for _ in 0..4 * CYCLE {
            if g.dragon().0 == 0 {
                return;
            }
            self.idle(g, 1);
        }
        panic!("no fresh sleep window");
    }
    /// Wait for the first tick of the doze window (eye cracks, warning).
    fn doze0(&mut self, g: &mut Dragon) {
        for _ in 0..4 * CYCLE {
            if g.dragon().0 != 1 {
                break;
            }
            self.idle(g, 1);
        }
        for _ in 0..4 * CYCLE {
            if g.dragon().0 == 1 {
                return;
            }
            self.idle(g, 1);
        }
        panic!("no doze window");
    }
}

/// Sprint west from wherever we are until the run resolves.
fn run_for_door(sim: &mut Sim, g: &mut Dragon) {
    for _ in 0..1500 {
        if !matches!(g.state(), MgState::Running) {
            return;
        }
        sim.axes(g, -1, 0);
    }
    panic!("never left the hall");
}

#[test]
fn banner_holds_the_room() {
    let mut sim = Sim::new();
    let mut g = Dragon::new(7);
    sim.idle(&mut g, 60);
    // title pixels on screen: some non-black ink in the title band
    let mut ink = 0;
    for y in 116..132u32 {
        for x in 82..240u32 {
            if sim.fb.get(x, y) != BLACK {
                ink += 1;
            }
        }
    }
    assert!(ink > 80, "the hoard banner never drew (ink {ink})");
    // the hall is frozen until the banner clears
    assert_eq!(g.pos(), (22.0, 190.0, -1));
    assert_eq!(g.dragon(), (0, 0.0, false));
    assert!(matches!(g.state(), MgState::Running));
    // even mashing every direction moves nothing yet
    for _ in 0..10 {
        sim.axes(&mut g, 1, 1);
    }
    assert_eq!(g.pos(), (22.0, 190.0, -1), "no running during the banner");
}

#[test]
fn weight_sloweth_the_thief() {
    let (mut sim, mut g) = (Sim::new(), Dragon::new(11));
    sim.idle(&mut g, PRESENT + 1);
    // sync on the beat in the beam's shadow, then measure a free stride:
    // ten ticks of easting, all of it inside the deep sleep
    sim.cycle0(&mut g); // west corner lieth beyond the beam's reach
    let (x0, _, _) = g.pos();
    for _ in 0..10 {
        sim.axes(&mut g, 1, 0);
    }
    let (x1, _, _) = g.pos();
    let free = x1 - x0;
    assert!(free > 15.0 && free < 17.0, "unencumbered stride is {free}");
    // pocket one sack in the deep sleep, then measure again
    sim.walk(&mut g, 150.0, 200.0);
    let (_, w, _) = g.loot();
    assert_eq!(w, 1, "walking onto a sack fills the pocket");
    assert!(!g.dragon().2, "a calm pickup must not wake him");
    let (y0, _, _) = g.pos();
    for _ in 0..10 {
        sim.axes(&mut g, 1, 0);
    }
    let (y1, _, _) = g.pos();
    let laden = y1 - y0;
    assert!(
        laden < free - 1.0 && laden > 13.0,
        "one sack: {laden} vs free {free}"
    );
}

#[test]
fn the_crown_is_heavy_as_three_sacks() {
    let (mut sim, mut g) = (Sim::new(), Dragon::new(5));
    sim.idle(&mut g, PRESENT + 1);
    sim.cycle0(&mut g);
    sim.walk(&mut g, 120.0, 176.0);
    sim.cycle0(&mut g);
    sim.walk(&mut g, 232.0, 150.0);
    let (carried, w, crown) = g.loot();
    assert!(crown, "the crown was never lifted");
    assert_eq!((carried, w), (36, 3), "crown = 36 points, 3 weight");
    assert!(g.dragon().2, "the cursed crown always wakes the dragon");
    assert!(matches!(g.state(), MgState::Running), "waking is not dying");
}

#[test]
fn cover_hideth_and_the_gaze_catcheth() {
    let (mut sim, mut g) = (Sim::new(), Dragon::new(3));
    sim.idle(&mut g, PRESENT + 1);
    sim.hide_at(&mut g, 120.0, 176.0);
    // two full cycles of sweeping eye wash over the pillar: nothing stirs
    sim.idle(&mut g, 2 * CYCLE + 20);
    assert!(!g.dragon().2, "the pillar keepeth its own");
    // step into the open and linger: the beam must find thee
    sim.cycle0(&mut g);
    sim.walk(&mut g, 200.0, 176.0);
    for _ in 0..4 * CYCLE {
        if g.dragon().2 {
            break;
        }
        sim.idle(&mut g, 1);
    }
    assert!(g.dragon().2, "the open eye catcheth the unhidden");
    assert!(
        matches!(g.state(), MgState::Running),
        "caught by the eye is awake, not dead"
    );
    assert_eq!(g.score(), 0, "waking him with empty hands wins nought");
}

#[test]
fn a_jangling_sack_in_the_doze_waketh_him() {
    let (mut sim, mut g) = (Sim::new(), Dragon::new(9));
    sim.idle(&mut g, PRESENT + 1);
    // creep to directly above bag2, one row outside its reach (dy 15)
    sim.hide_at(&mut g, 120.0, 176.0);
    sim.cycle0(&mut g);
    sim.walk(&mut g, 200.0, 184.0); // the y184 corridor misses crown + sacks
    sim.walk(&mut g, 266.0, 184.0);
    assert_eq!(g.loot().1, 0, "passing two rows clear pocketeth nothing");
    assert!(!g.dragon().2);
    // creep UP into bag2's reach while the eye is cracked: coins jingle
    sim.doze0(&mut g);
    for _ in 0..4 {
        sim.axes(&mut g, 0, -1);
        if g.loot().1 > 0 {
            break;
        }
    }
    assert_eq!(g.loot(), (18, 1, false), "bag2 taken in the doze");
    assert!(g.dragon().2, "a near sack jangled in the doze");
}

#[test]
fn a_still_grab_in_the_sleep_is_silent() {
    let (mut sim, mut g) = (Sim::new(), Dragon::new(13));
    sim.idle(&mut g, PRESENT + 1);
    sim.hide_at(&mut g, 120.0, 176.0);
    sim.cycle0(&mut g);
    sim.walk(&mut g, 150.0 - 26.0, 176.0); // clear of bag0 on two axes
    sim.doze0(&mut g);
    for _ in 0..16 {
        // drift down-right until within reach of bag0 (doze window is 20)
        sim.axes(&mut g, 1, 1);
        if g.loot().1 > 0 {
            break;
        }
    }
    let (carried, w, _) = g.loot();
    assert_eq!(carried, 18, "bag0 taken");
    assert_eq!(w, 1);
    assert!(!g.dragon().2, "a NON-near sack never jangles");
}

#[test]
fn even_a_near_sack_is_quiet_during_sleep() {
    let (mut sim, mut g) = (Sim::new(), Dragon::new(17));
    sim.idle(&mut g, PRESENT + 1);
    sim.hide_at(&mut g, 120.0, 176.0);
    sim.cycle0(&mut g);
    sim.walk(&mut g, 222.0, 197.0); // beside bag1, one row outside reach
    assert_eq!(g.dragon().0, 0, "leg still inside the sleep window");
    for _ in 0..3 {
        // near sack, but he is deep asleep (dy crosses 14 in two steps)
        sim.axes(&mut g, 0, 1);
        if g.loot().1 >= 1 {
            break;
        }
    }
    assert!(g.loot().1 >= 1, "bag1 pocketed");
    assert!(!g.dragon().2, "only the DOZE jangle wakes him");
}

#[test]
fn the_fire_keepeth_a_standing_thief() {
    let (mut sim, mut g) = (Sim::new(), Dragon::new(21));
    sim.idle(&mut g, PRESENT + 1);
    sim.cycle0(&mut g);
    sim.walk(&mut g, 120.0, 176.0);
    sim.cycle0(&mut g);
    sim.walk(&mut g, 232.0, 150.0); // crown: wakes him, and then stand there
    for _ in 0..300 {
        if !matches!(g.state(), MgState::Running) {
            break;
        }
        sim.stand_still(&mut g);
    }
    assert!(!g.escaped());
    assert_eq!(g.state(), MgState::Lost, "the flame finds the stationary");
    assert_eq!(g.score(), 0, "caught inside, the loot stayeth behind");
}

#[test]
fn the_mark_telegrapheth_before_the_flame() {
    let (mut sim, mut g) = (Sim::new(), Dragon::new(23));
    sim.idle(&mut g, PRESENT + 1);
    sim.cycle0(&mut g);
    sim.walk(&mut g, 120.0, 176.0);
    sim.cycle0(&mut g);
    sim.walk(&mut g, 232.0, 166.0); // one row above the crown disc
    assert!(!g.dragon().2, "the crown sleepeth yet");
    for _ in 0..4 {
        // step INTO the disc: the waking tick is now beyond doubt
        sim.axes(&mut g, 0, -1);
        if g.dragon().2 {
            break;
        }
    }
    assert!(g.dragon().2, "the crown waketh upon the grab tick");
    // roar (23 ticks left after the waking tick) + stalk (45) -> ember
    let mut t = 0;
    while g.volley().0 == 0 && t < 90 {
        sim.stand_still(&mut g);
        t += 1;
    }
    assert_eq!(t, 23 + 45, "first ember lands when the roar burns out");
    let (st, tx) = g.volley();
    assert_eq!(st, 1);
    assert!(
        (tx - 232.0).abs() < 2.0,
        "he marketh the standing thief, saw {tx}"
    );
    // flame follows exactly MARK_T (16) ticks after the mark
    sim.idle(&mut g, 15);
    assert_eq!(g.volley().0, 1, "still telegraphing one tick shy");
    sim.stand_still(&mut g);
    assert_eq!(g.volley().0, 2, "ember bursts into flame");
    assert!(matches!(g.state(), MgState::Running), "the flame kills next tick");
    sim.stand_still(&mut g);
    assert_eq!(g.state(), MgState::Lost, "standing in the fire is dying");
}

#[test]
fn fleeing_the_mark_outspeedeth_the_fire() {
    let (mut sim, mut g) = (Sim::new(), Dragon::new(29));
    sim.idle(&mut g, PRESENT + 1);
    sim.cycle0(&mut g);
    sim.walk(&mut g, 120.0, 176.0);
    sim.cycle0(&mut g);
    sim.walk(&mut g, 232.0, 150.0); // crown: awake, then run
    run_for_door(&mut sim, &mut g); // every volley marks where thou WAST
    assert!(g.escaped(), "a running thief outrunneth every volley");
    assert_eq!(g.state(), MgState::Won(Tier::Bronze));
    assert_eq!(g.score(), 36 + 10, "crown haul plus the door bonus");
}

#[test]
fn empty_hands_escape_winneth_nought() {
    let (mut sim, mut g) = (Sim::new(), Dragon::new(31));
    sim.idle(&mut g, PRESENT + 1);
    run_for_door(&mut sim, &mut g); // straight back out the way we came
    assert!(g.escaped());
    assert_eq!(g.state(), MgState::Lost, "10 points is no victory");
    assert_eq!(g.score(), 10);
}

#[test]
fn the_perfect_heist_carryeth_the_hoard() {
    let (mut sim, mut g) = (Sim::new(), Dragon::new(37));
    sim.idle(&mut g, PRESENT + 1);
    // The full hoard waketh him - the mark flyeth all the while. Never
    // stand still, ne'er retrace a marked row: the flame striketh where
    // thou WAST sixteen ticks agone, and 16 * stride outmatcheth it.
    sim.hide_at(&mut g, 120.0, 176.0);
    sim.cycle0(&mut g);
    sim.walk(&mut g, 266.0, 172.0); // bag2: easternmost first
    assert_eq!(g.loot().1, 1, "bag2 pocketed");
    sim.walk(&mut g, 208.0, 212.0); // bag1
    assert_eq!(g.loot().1, 2, "bag1 pocketed");
    sim.walk(&mut g, 150.0, 200.0); // bag0
    assert_eq!(g.loot(), (54, 3, false));
    sim.walk(&mut g, 232.0, 163.0); // a row above the cursed crown
    sim.axes(&mut g, 0, -1); // heavy boots creep slow: two ticks to lean in
    sim.axes(&mut g, 0, -1);
    assert!(g.dragon().2, "the crown waketh him");
    assert_eq!(g.loot(), (90, 6, true));
    run_for_door(&mut sim, &mut g);
    assert!(g.escaped(), "the whole hoard, and out alive");
    assert_eq!(g.state(), MgState::Won(Tier::Gold));
    assert_eq!(g.score(), 100, "90 in loot + 10 out the door");
}

#[test]
fn the_door_keepeth_its_own_row() {
    let (mut sim, mut g) = (Sim::new(), Dragon::new(41));
    sim.idle(&mut g, PRESENT + 1);
    // hug the floor line PAST the door mouth: the west wall alone is no exit
    sim.walk(&mut g, 40.0, DOOR_ROW);
    sim.walk(&mut g, 40.0, Y_MAX);
    sim.walk(&mut g, X_MIN, Y_MAX);
    sim.idle(&mut g, 5);
    assert!(!g.escaped(), "the floor line is not a door");
    for _ in 0..60 {
        // creep north hugging the wall until the keepin' row opeth
        if !matches!(g.state(), MgState::Running) {
            break;
        }
        sim.axes(&mut g, 0, -1);
    }
    sim.idle(&mut g, 5);
    assert!(g.escaped(), "the west door openeth at the keepin' row");
}
