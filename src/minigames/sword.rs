//! mg-sword — DUEL (SPEC 6 #3): best-of-five parry and riposte. The
//! herald shows your foe's opening — a raised blade HIGH, a level point
//! MID, a low guard LOW. Answer the opening with the matching guard
//! (UP / RIGHT / DOWN, or click the band on your half) while he winds up.
//! From round three on he may feint: the blade dips early — the honest
//! tell is the GOLD FLASH at his hip, which always marks the true line.
//! A clean parry staggers him; press SPACE (or click) inside the riposte
//! window — not before it, the meter shows the gap — to win the pass.
//!
//! Scoring is normalized to 0..=100: 20 for every pass won, +8 when a
//! pass is won without a flinch or a whiff, and +16 for finishing the
//! whole bout unhurt. Three clean passes make the perfect script land
//! exactly on 100, one tick above the Gold line (90).

use super::framework::{resolve, tap_action, MgState, Minigame, SportSpec, Tier, SPORTS};
use crate::engine::fonts::draw_text_shadow;
use crate::engine::gfx::{
    Framebuffer, BLACK, BLOOD, BROWN, CREAM, DARKGRAY, GOLD, GRASS, GRAY, LINEN, NAVY, NORMAN_RED,
    RED, SAXON_BLUE, SILVER, TAN, WHITE, YELLOW,
};
use crate::engine::input::{Input, KEY_DOWN, KEY_RIGHT, KEY_SPACE, KEY_UP};

pub const ROUNDS_MAX: u32 = 5;
pub const PTS_TO_WIN: u32 = 3;
pub const EXCH_MAX: u32 = 6; // exchanges per round before it is called even

pub const PRESENT: u32 = 80; // banner + instructions
pub const HIT: u32 = 40; // clash beat before the next round
pub const END: u32 = 50; // verdict panel hold
pub const RECOVER: u32 = 20; // foe shrugs off a missed window
pub const RECOIL: u32 = 44; // foe stunned after a clean parry

pub const FLOOR_Y: f32 = 206.0;
pub const PLAYER_X: i32 = 96;
pub const FOE_X: i32 = 208;

/// One parry band (y range for clicks on our half).
pub const BANDS: [(i32, i32); 3] = [(96, 140), (140, 174), (174, 206)];

/// Points per SPEC 6 #3 scaling.
pub const PASS_PTS: u32 = 20;
pub const CLEAN_PTS: u32 = 8;
pub const UNHURT_PTS: u32 = 16;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lane {
    High,
    Mid,
    Low,
}

impl Lane {
    pub fn key(self) -> u32 {
        match self {
            Lane::High => KEY_UP,
            Lane::Mid => KEY_RIGHT,
            Lane::Low => KEY_DOWN,
        }
    }
    pub fn index(self) -> usize {
        match self {
            Lane::High => 0,
            Lane::Mid => 1,
            Lane::Low => 2,
        }
    }
    pub fn from_index(i: usize) -> Self {
        match i % 3 {
            0 => Lane::High,
            1 => Lane::Mid,
            _ => Lane::Low,
        }
    }
    pub fn word(self) -> &'static str {
        match self {
            Lane::High => "HIGH",
            Lane::Mid => "MID",
            Lane::Low => "LOW",
        }
    }
    /// Blade height for a pose in this lane.
    pub fn pose_y(self) -> i32 {
        match self {
            Lane::High => 118,
            Lane::Mid => 152,
            Lane::Low => 188,
        }
    }
}

/// Which guard key a scripted run must tap for a lane (harness contract).
pub fn key_for(l: Lane) -> u32 {
    l.key()
}

/// One foe opening: the guard he shows, the true line if he lies, and
/// how long the wind-up runs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Attack {
    /// Guard shown at the start of the wind-up (the decoy lane if any).
    pub tell: Lane,
    /// `Some(f)` — the gold flash marks `f`; the blade dips to it one
    /// `FEINT_SWITCH` before the strike.
    pub feint: Option<Lane>,
    /// Wind-up ticks (the reaction window).
    pub windup: u32,
}

impl Attack {
    pub fn final_lane(&self) -> Lane {
        self.feint.unwrap_or(self.tell)
    }
}

const W_FULL: u32 = 46;
const W_FAST: u32 = 38;
pub const FEINT_SWITCH: u32 = 12;

pub fn windup_for(round: u32) -> u32 {
    if round >= 2 {
        W_FAST
    } else {
        W_FULL
    }
}

fn mix(mut x: u64) -> u64 {
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

/// The foe's entire bout for one seed: five openings, at most one feint
/// per round and never before round three. Tests and the browser harness
/// script their parries off this — the single source of the pattern.
pub fn plan(seed: u64) -> [Attack; ROUNDS_MAX as usize] {
    let mut out = [Attack {
        tell: Lane::Mid,
        feint: None,
        windup: W_FULL,
    }; ROUNDS_MAX as usize];
    for (r, a) in out.iter_mut().enumerate() {
        let h = mix(seed ^ (r as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let tell = Lane::from_index((h % 3) as usize);
        let mut feint = None;
        if (h / 3).is_multiple_of(5) && r >= 2 {
            let f = Lane::from_index((((h >> 7) % 2) + 1) as usize + tell.index());
            feint = Some(if f.index() == tell.index() {
                Lane::from_index(tell.index() + 1)
            } else {
                f
            });
        }
        *a = Attack {
            tell,
            feint,
            windup: windup_for(r as u32),
        };
    }
    out
}

/// True while a riposte lands: `since` frames after the clean parry.
/// Shared by the live game, the scripted tests and the browser solver.
pub fn riposte_ok(since: u32) -> bool {
    (14..26).contains(&since)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Present,
    Windup,
    Recoil, // foe staggered: riposte window open
    Whiff,  // struck early: foe is recovering
    Hit,    // we were struck
    RoundEnd,
    Done,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pass {
    Pending,
    Parried,
    Landed,
    Flinched,
    Struck,
    Whiffed,
    Even,
}

impl Pass {
    fn word(self) -> &'static str {
        match self {
            Pass::Pending => "",
            Pass::Parried => "PARRY",
            Pass::Landed => "RIPOSTE",
            Pass::Flinched => "FALSE GUARD",
            Pass::Struck => "STRUCK",
            Pass::Whiffed => "TOO EARLY",
            Pass::Even => "EVEN",
        }
    }
    fn color(self) -> u8 {
        match self {
            Pass::Landed => GOLD,
            Pass::Parried => WHITE,
            Pass::Struck | Pass::Flinched => RED,
            Pass::Whiffed | Pass::Even => YELLOW,
            Pass::Pending => GRAY,
        }
    }
}

/// One bout. Pure logic; the host owns pause and the cheer timer.
pub struct Sword {
    plan: [Attack; ROUNDS_MAX as usize],
    phase: Phase,
    phase_t: u32,
    hint_t: u32, // ticks since play opened: fades the bottom strip (T-UX)
    round: u32,  // 0-based round index
    exch: u32,   // exchanges used in the current round
    since_parry: u32,
    flinch: bool, // parried the decoy lane on the strike that hurt us
    hit: bool,    // struck at least once this match
    clean: [bool; ROUNDS_MAX as usize],
    won: [bool; ROUNDS_MAX as usize],
    guard: Option<Lane>,
    foe_pts: u32,
    mine: u32,
    last: Pass,
    banner: u32,
    score: u32,
    state: MgState,
}

impl Sword {
    pub fn new(seed: u64) -> Self {
        Self {
            plan: plan(seed),
            phase: Phase::Present,
            hint_t: 0,
            phase_t: 0,
            round: 0,
            exch: 0,
            since_parry: 0,
            flinch: false,
            hit: false,
            clean: [true; ROUNDS_MAX as usize],
            won: [false; ROUNDS_MAX as usize],
            guard: None,
            foe_pts: 0,
            mine: 0,
            last: Pass::Pending,
            banner: 0,
            score: 0,
            state: MgState::Running,
        }
    }

    fn spec(&self) -> &'static SportSpec {
        SPORTS.iter().find(|s| s.id == self.id()).unwrap()
    }

    // ---- public readouts for HUD, tests and the browser harness ----------

    pub fn round_no(&self) -> usize {
        self.round as usize
    }
    pub fn score_pts(&self) -> (u32, u32) {
        (self.mine, self.foe_pts)
    }
    /// The line the foe is truly throwing at us (feint already resolved).
    pub fn lane_now(&self) -> Lane {
        self.plan[self.round as usize].final_lane()
    }
    /// The blade pose currently shown: the decoy until the switch tick,
    /// then the true line.
    pub fn pose(&self) -> Lane {
        let a = self.plan[self.round as usize];
        if self.phase == Phase::Windup
            && a.feint.is_some()
            && a.windup - self.phase_t > FEINT_SWITCH
        {
            a.tell
        } else {
            a.final_lane()
        }
    }
    /// Ticks left before the blade lands (the reaction clock).
    pub fn ticks_left(&self) -> u32 {
        if self.phase == Phase::Windup {
            self.plan[self.round as usize].windup - self.phase_t
        } else {
            0
        }
    }
    /// Gold flash at the foe's hip: the honest tell during a feint.
    pub fn feint_tell(&self) -> bool {
        let a = self.plan[self.round as usize];
        let since = a.windup.saturating_sub(self.phase_t);
        a.feint.is_some() && self.phase == Phase::Windup && since <= FEINT_SWITCH + 6
    }
    /// The guard currently held, if any.
    pub fn guard(&self) -> Option<Lane> {
        self.guard
    }
    pub fn riposte_ticks(&self) -> u32 {
        self.since_parry
    }
    pub fn window_open(&self) -> bool {
        self.phase == Phase::Recoil && riposte_ok(self.since_parry)
    }
    pub fn last_pass(&self) -> Pass {
        self.last
    }
    pub fn unhurt(&self) -> bool {
        !self.hit
    }
    pub fn won_round(&self, r: usize) -> bool {
        self.won[r]
    }
    pub fn clean_round(&self, r: usize) -> bool {
        self.clean[r]
    }
    /// True while an exchange is open: wind-up (can guard) or stagger
    /// (can riposte).
    pub fn exchange(&self) -> bool {
        matches!(self.phase, Phase::Windup | Phase::Recoil)
    }
    pub fn done(&self) -> bool {
        self.phase == Phase::Done
    }
    /// The bout's final tally — the same formula `finish` banks, exposed
    /// so tests and the harness can reason about it mid-run.
    pub fn final_score(&self) -> u32 {
        let mut s = 0u32;
        for r in 0..ROUNDS_MAX as usize {
            if self.won[r] {
                s += PASS_PTS;
                if self.clean[r] {
                    s += CLEAN_PTS;
                }
            }
        }
        if self.mine > self.foe_pts && !self.hit {
            s += UNHURT_PTS;
        }
        s.min(100)
    }

    // ---- game flow --------------------------------------------------------

    fn begin_exchange(&mut self) {
        self.phase = Phase::Windup;
        self.phase_t = 0;
        self.exch += 1;
        self.flinch = false;
        self.guard = None;
        self.since_parry = 0;
        self.banner = 0;
    }

    fn strike(&mut self) {
        // The blade lands. A guard held at the true line parries clean;
        // the decoy line is a flinch; nothing at all is a hit.
        let a = self.plan[self.round as usize];
        match self.guard {
            Some(g) if g == a.final_lane() => {
                self.since_parry = 0;
                self.phase = Phase::Recoil;
                self.last = Pass::Parried;
                self.banner = 18;
            }
            Some(_) => {
                self.flinch = true;
                self.clean[self.round as usize] = false;
                self.fall();
            }
            None => {
                self.clean[self.round as usize] = false;
                self.fall();
            }
        }
    }

    fn fall(&mut self) {
        self.hit = true;
        self.foe_pts += 1;
        self.last = if self.flinch {
            Pass::Flinched
        } else {
            Pass::Struck
        };
        self.banner = 30;
        self.phase = Phase::Hit;
        self.phase_t = 0;
    }

    fn try_riposte(&mut self) {
        if riposte_ok(self.since_parry) {
            self.mine += 1;
            self.won[self.round as usize] = true;
            self.last = Pass::Landed;
            self.banner = 24;
            self.phase = Phase::RoundEnd;
            self.phase_t = 0;
        } else if self.since_parry < 14 {
            self.clean[self.round as usize] = false;
            self.last = Pass::Whiffed;
            self.banner = 20;
            self.phase = Phase::Whiff;
            self.phase_t = 0;
        }
        // past the window there is nothing to press: RECOVER runs out.
    }

    fn next_exchange(&mut self) {
        if self.exch >= EXCH_MAX {
            self.last = Pass::Even;
            self.banner = 24;
            self.phase = Phase::RoundEnd;
            self.phase_t = 0;
        } else {
            self.begin_exchange();
        }
    }

    fn advance_after(&mut self) {
        if self.mine >= PTS_TO_WIN || self.foe_pts >= PTS_TO_WIN || self.round + 1 >= ROUNDS_MAX {
            self.finish();
        } else {
            self.round += 1;
            self.exch = 0;
            self.begin_exchange();
        }
    }

    fn finish(&mut self) {
        self.score = self.final_score();
        self.state = resolve(self.score, self.spec());
        self.phase = Phase::Done;
        self.phase_t = 0;
    }
}

impl Minigame for Sword {
    fn id(&self) -> &'static str {
        "sword"
    }

    fn score(&self) -> u32 {
        self.score
    }

    fn state(&self) -> MgState {
        self.state
    }

    fn tick(&mut self, dt: f32, input: &Input, fb: &mut Framebuffer) {
        if dt <= 0.0 {
            self.paint(fb);
            return;
        }
        self.phase_t += 1;
        if self.phase != Phase::Present {
            self.hint_t += 1; // T-UX: play-start clock for the bottom strip
        }
        if self.banner > 0 {
            self.banner -= 1;
        }
        match self.phase {
            Phase::Present => {
                if self.phase_t >= PRESENT {
                    self.begin_exchange();
                }
            }
            Phase::Windup => {
                let a = self.plan[self.round as usize];
                for l in [Lane::High, Lane::Mid, Lane::Low] {
                    if input.key_pressed(l.key()) {
                        self.guard = Some(l);
                    }
                }
                if input.pressed() {
                    let (mx, my, _) = input.mouse();
                    if mx < 160 {
                        self.guard = Self::band_at(my);
                    }
                }
                if self.phase_t >= a.windup {
                    self.strike();
                }
            }
            Phase::Recoil => {
                self.since_parry += 1;
                if tap_action(input, KEY_SPACE) || input.pressed() {
                    self.try_riposte();
                }
                if self.phase == Phase::Recoil && self.since_parry >= RECOIL {
                    self.last = Pass::Even;
                    self.banner = 20;
                    self.phase = Phase::RoundEnd;
                    self.phase_t = 0;
                }
            }
            Phase::Whiff => {
                if self.phase_t >= RECOVER {
                    self.next_exchange();
                }
            }
            Phase::Hit | Phase::RoundEnd => {
                if self.phase_t >= HIT {
                    self.advance_after();
                }
            }
            Phase::Done => {}
        }
        self.paint(fb);
    }
}

impl Sword {
    fn band_at(y: i32) -> Option<Lane> {
        BANDS
            .iter()
            .position(|(lo, hi)| y >= *lo && y < *hi)
            .map(Lane::from_index)
    }

    fn paint(&self, fb: &mut Framebuffer) {
        // tilt: bunting, crowd row, sandy ground -------------------------
        fb.fill_rect(0, 0, 320, 12, BLACK);
        fb.fill_rect(0, 12, 320, (FLOOR_Y as i32 - 12) as u32, NAVY);
        crate::minigames::framework::dusk_sky(fb, 12, FLOOR_Y as i32);
        for x in (8..312).step_by(24) {
            let c = if (x / 24) % 2 == 0 {
                SAXON_BLUE
            } else {
                NORMAN_RED
            };
            fb.fill_rect(x, 12, 12, 4, c);
        }
        fb.fill_rect(0, 30, 320, 14, DARKGRAY);
        for x in (4..316).step_by(6) {
            fb.fill_rect(x, 32, 3, 6, if (x / 6) % 3 == 0 { TAN } else { GRAY });
        }
        fb.fill_rect(0, FLOOR_Y as i32, 320, 50, GRASS);
        fb.fill_rect(0, FLOOR_Y as i32, 320, 2, BROWN);
        crate::minigames::framework::skyline(fb, FLOOR_Y as i32);
        // click bands on our half -----------------------------------------
        if self.phase == Phase::Windup {
            for (i, (lo, hi)) in BANDS.iter().enumerate() {
                let hot = self.guard == Some(Lane::from_index(i));
                fb.fill_rect(
                    2,
                    *lo,
                    150,
                    (hi - lo) as u32,
                    if hot { GOLD } else { BLACK },
                );
                draw_text_shadow(
                    fb,
                    6,
                    lo + 2,
                    Lane::from_index(i).word(),
                    if hot { BLACK } else { GRAY },
                    BLACK,
                );
            }
        }
        // the two knights ---------------------------------------------------
        self.knight(fb, PLAYER_X, SILVER, self.guard, true);
        self.knight(fb, FOE_X, BLOOD, Some(self.pose()), false);
        // feint tell: gold flash at the foe's hip ----------------------------
        if self.feint_tell() && (self.phase_t / 2).is_multiple_of(2) {
            fb.fill_rect(FOE_X - 2, 168, 5, 5, GOLD);
        }
        // clash sparks on the banner beat ------------------------------------
        if self.banner > 0 && matches!(self.last, Pass::Landed | Pass::Struck | Pass::Flinched) {
            let py = if self.last == Pass::Landed { 150 } else { 158 };
            let k = ((self.banner % 3) * 2) as i32;
            for (ox, oy) in [(-3 - k, 0), (3 + k, 0), (0, -3 - k), (0, 3 + k)] {
                fb.fill_rect(150 + ox, py + oy, 2, 2, YELLOW);
            }
        }
        // HUD -----------------------------------------------------------------
        // HUD counters live at y>=14: the host band owns rows 0-11.
        draw_text_shadow(
            fb,
            60,
            14,
            &format!(
                "ROUND {}/5",
                (self.round as usize).min(ROUNDS_MAX as usize - 1) + 1
            ),
            CREAM,
            BLACK,
        );
        for i in 0..PTS_TO_WIN {
            fb.fill_rect(
                150 + i as i32 * 8,
                14,
                6,
                6,
                if i < self.mine { SAXON_BLUE } else { DARKGRAY },
            );
        }
        for i in 0..PTS_TO_WIN {
            fb.fill_rect(
                296 - i as i32 * 8,
                14,
                6,
                6,
                if i < self.foe_pts {
                    NORMAN_RED
                } else {
                    DARKGRAY
                },
            );
        }
        if self.hit {
            draw_text_shadow(fb, 224, 14, "HURT", RED, BLACK);
        }
        if self.banner > 0 && !self.last.word().is_empty() && self.phase != Phase::Done {
            draw_text_shadow(fb, 116, 96, self.last.word(), self.last.color(), BLACK);
        }
        // riposte meter: SPACE once the bar crosses the gold gap --------------
        if self.phase == Phase::Recoil {
            fb.fill_rect(128, 230, 64, 6, DARKGRAY);
            let w = (self.since_parry.min(40) * 64 / 40).max(1);
            let c = if self.window_open() { GOLD } else { SILVER };
            fb.fill_rect(129, 231, w, 4, c);
            let g0 = 14 * 64 / 40;
            let g1 = 26 * 64 / 40;
            fb.fill_rect(129 + g0, 229, (g1 - g0) as u32, 8, BLACK);
            fb.fill_rect(129 + g0, 229, 1, 8, GOLD);
            if self.hint_t < 180 {
                // T-UX: the gap prompt fades ~3 s into the bout
                draw_text_shadow(fb, 118, 240, "STRIKE IN THE GAP", GRAY, BLACK);
            }
        }
        match self.phase {
            Phase::Present => {
                fb.fill_rect(36, 92, 248, 68, BLACK);
                fb.rect(36, 92, 248, 68, GOLD);
                crate::ui::kit::centered(fb, 100, "THE DUEL", GOLD, BLACK);
                crate::ui::kit::centered(fb, 114, "MATCH HIS GUARD: UP HIGH,", LINEN, BLACK);
                crate::ui::kit::centered(fb, 124, "RIGHT MID, DOWN LOW -", LINEN, BLACK);
                crate::ui::kit::centered(fb, 134, "OR CLICK YOUR BANDED HALF.", LINEN, BLACK);
                crate::ui::kit::centered(fb, 146, "SPACE STRIKES THE RIPOSTE", CREAM, BLACK);
            }
            Phase::Done => {
                fb.fill_rect(52, 84, 216, 84, BLACK);
                fb.rect(52, 84, 216, 84, GOLD);
                let verdict = match self.state {
                    MgState::Won(Tier::Gold) => "CHAMPION!",
                    MgState::Won(Tier::Silver) => "A NOBLE BOUT",
                    MgState::Won(Tier::Bronze) => "THE CROWD NODS",
                    _ => "THE FIELD IS HIS",
                };
                draw_text_shadow(fb, 108, 94, verdict, GOLD, BLACK);
                draw_text_shadow(
                    fb,
                    108,
                    110,
                    &format!("PASSES {} - {}", self.mine, self.foe_pts),
                    CREAM,
                    BLACK,
                );
                draw_text_shadow(fb, 120, 126, &format!("SCORE {}", self.score), WHITE, BLACK);
                if self.unhurt() {
                    draw_text_shadow(fb, 92, 142, "UNHURT - THE FIELD LAUDS", LINEN, BLACK);
                }
            }
            _ => {}
        }
    }

    /// One knight at `x` (feet on FLOOR), tunic `tunic`, blade held in
    /// `blade` lane. Player faces right, foe faces left.
    fn knight(&self, fb: &mut Framebuffer, x: i32, tunic: u8, blade: Option<Lane>, ours: bool) {
        let dir: i32 = if ours { 1 } else { -1 };
        let stagger = if ours && self.phase == Phase::Hit && (self.phase_t / 3).is_multiple_of(2) {
            2
        } else if !ours && self.phase == Phase::Recoil && (self.phase_t / 3).is_multiple_of(2) {
            -2 * dir
        } else {
            0
        };
        let x = x + stagger;
        // legs, torso, helm
        fb.fill_rect(x - 3, 186, 3, 20, BROWN);
        fb.fill_rect(x + 1, 186, 3, 20, BROWN);
        fb.fill_rect(x - 5, 152, 11, 34, tunic);
        fb.fill_rect(x - 4, 138, 9, 12, SILVER);
        fb.fill_rect(x - 4, 138, 9, 3, if ours { SAXON_BLUE } else { NORMAN_RED });
        // shield arm on the far side
        let sx = x - dir * 8;
        fb.fill_rect(
            sx - 2,
            156,
            5,
            12,
            if ours { SAXON_BLUE } else { NORMAN_RED },
        );
        // blade: grip at the shoulder, edge dots to the lane height, point
        let bx = x + dir * 8;
        fb.fill_rect(bx - 1, 146, 3, 6, TAN);
        match blade {
            Some(l) => {
                let ty = l.pose_y();
                for i in 0..=6 {
                    let t = i as f32 / 6.0;
                    let px = bx + dir * (2 + (t * 12.0) as i32);
                    let py = (148.0 + (ty - 148) as f32 * t) as i32;
                    fb.fill_rect(px, py, 2, 2, WHITE);
                }
                let px = if dir > 0 { bx + 14 } else { bx - 20 };
                fb.fill_rect(px, ty - 1, 6, 3, WHITE);
            }
            None => {
                fb.fill_rect(bx + dir * 2, 150, 2, 14, WHITE); // blade down
            }
        }
    }
}
