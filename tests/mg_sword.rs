//! T504: mg-sword — pattern determinism, telegraph/feint reading, the
//! riposte window, and the SPEC 6 #3 tier scaling. Driven through the
//! same Input and Framebuffer the web host feeds it, like the
//! joust/catapult suites.

use defender_of_the_crown::engine::gfx::Framebuffer;
use defender_of_the_crown::engine::input::{Input, Raw, KEY_SPACE};
use defender_of_the_crown::minigames::framework::{MgState, Minigame, Tier};
use defender_of_the_crown::minigames::sword::{
    key_for, plan, riposte_ok, windup_for, Lane, Pass, Sword, EXCH_MAX,
};

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
    /// One 60 Hz frame; queued events land as this frame's edges.
    fn frame(&mut self, g: &mut Sword, events: &[Raw]) {
        for e in events {
            self.inp.push(*e);
        }
        self.frame += 1;
        self.inp.begin_frame(self.frame);
        g.tick(1.0 / 60.0, &self.inp, &mut self.fb);
    }
    /// Run out the intro banner.
    fn wait_present(&mut self, g: &mut Sword) {
        for _ in 0..85 {
            self.frame(g, &[]);
        }
        assert!(g.exchange(), "the duel never opened");
    }
    /// Tap a guard and hold it through the strike.
    fn parry(&mut self, g: &mut Sword, l: Lane) {
        let k = key_for(l);
        self.frame(
            g,
            &[Raw::Key {
                code: k,
                pressed: true,
            }],
        );
        assert_eq!(g.guard(), Some(l));
        self.frame(
            g,
            &[Raw::Key {
                code: k,
                pressed: false,
            }],
        );
    }
    /// Wait for the blade to land and the foe to stagger.
    fn wait_stagger(&mut self, g: &mut Sword) {
        for _ in 0..80 {
            if g.riposte_ticks() > 0 {
                return;
            }
            assert!(g.exchange(), "the parry never staggered the foe");
            self.frame(g, &[]);
        }
        panic!("no stagger after a clean parry");
    }
    /// Wait for the gold gap and strike inside it.
    fn riposte(&mut self, g: &mut Sword) {
        self.wait_stagger(g);
        for _ in 0..30 {
            if g.window_open() {
                break;
            }
            self.frame(g, &[]);
        }
        assert!(
            g.window_open(),
            "window never opened: ticks={} pass={:?} pts={:?}",
            g.riposte_ticks(),
            g.last_pass(),
            g.score_pts()
        );
        self.frame(
            g,
            &[Raw::Key {
                code: KEY_SPACE,
                pressed: true,
            }],
        );
        self.frame(
            g,
            &[Raw::Key {
                code: KEY_SPACE,
                pressed: false,
            }],
        );
    }
    /// Parry the true line, riposte in the gap; the pass is ours.
    fn win_pass(&mut self, g: &mut Sword) {
        let lane = g.lane_now();
        self.parry(g, lane);
        self.riposte(g);
        assert_eq!(g.last_pass(), Pass::Landed);
    }
    /// Run frames until the round index changes or the bout ends.
    fn wait_round(&mut self, g: &mut Sword, round: usize) {
        for _ in 0..3000 {
            self.frame(g, &[]);
            if g.done() || g.round_no() > round {
                return;
            }
        }
        panic!("round {round} never resolved");
    }
}

#[test]
fn the_foe_plans_the_same_bout_from_the_same_seed() {
    for seed in [0u64, 1, 7, 0xC0FFEE, u64::MAX] {
        let a = plan(seed);
        let b = plan(seed);
        assert_eq!(a, b, "bout {seed} must be pure");
        for (r, atk) in a.iter().enumerate() {
            assert_eq!(atk.windup, windup_for(r as u32), "round {r} windup");
            if r < 2 {
                assert_eq!(atk.feint, None, "no feints before round three");
            }
            if let Some(f) = atk.feint {
                assert_ne!(f, atk.tell, "a feint must lie about the line");
            }
        }
    }
}

#[test]
fn a_pattern_perfect_script_wins_3_0_unhurt_for_gold() {
    let mut s = Sim::new();
    let seed = 0xC0FFEE;
    let mut g = Sword::new(seed);
    let script = plan(seed);
    s.wait_present(&mut g);
    while !g.done() {
        let r = g.round_no();
        assert!(r < script.len());
        s.win_pass(&mut g);
        assert!(g.won_round(r) && g.clean_round(r), "pass {r} was not clean");
        s.wait_round(&mut g, r);
    }
    assert_eq!(g.score_pts(), (3, 0));
    assert!(g.unhurt());
    assert_eq!(g.score(), 100, "perfect script is the 100-point run");
    assert_eq!(g.state(), MgState::Won(Tier::Gold));
}

#[test]
fn an_open_guard_eats_the_blade_and_the_crowd_loses_faith() {
    let mut s = Sim::new();
    let mut g = Sword::new(11);
    s.wait_present(&mut g);
    let mut struck = 0;
    while !g.done() {
        let before = g.score_pts().1;
        let r = g.round_no();
        s.wait_round(&mut g, r);
        if g.score_pts().1 > before {
            struck += 1;
        }
    }
    assert_eq!(g.score_pts().1, 3, "three unguarded passes end the bout");
    assert_eq!(struck, 3);
    assert!(!g.unhurt());
    assert_eq!(g.score(), 0);
    assert_eq!(g.state(), MgState::Lost, "a whipping pays nothing");
}

#[test]
fn a_parry_never_struck_calls_the_pass_even_and_hurts_nobody() {
    let mut s = Sim::new();
    let mut g = Sword::new(5);
    s.wait_present(&mut g);
    let r = g.round_no();
    let lane = g.lane_now();
    s.parry(&mut g, lane);
    // sit on the parry: the foe recovers, the even pass closes the round
    for _ in 0..160 {
        s.frame(&mut g, &[]);
    }
    assert_eq!(g.last_pass(), Pass::Even);
    assert_eq!(g.score_pts(), (0, 0), "a parry alone scores nothing");
    assert!(
        g.done() || g.round_no() > r,
        "the even pass still ended the round"
    );
    assert!(!g.won_round(r), "even passes win nothing");
    assert!(g.unhurt());
}

#[test]
fn an_early_strike_whiffs_yet_the_pass_can_still_be_taken() {
    let mut s = Sim::new();
    let mut g = Sword::new(9);
    s.wait_present(&mut g);
    let r = g.round_no();
    let lane = g.lane_now();
    s.parry(&mut g, lane);
    // let the blade land, then jump the gun inside the stagger
    s.wait_stagger(&mut g);
    assert!(!g.window_open() && g.exchange());
    assert!(!riposte_ok(g.riposte_ticks()));
    s.frame(
        &mut g,
        &[Raw::Key {
            code: KEY_SPACE,
            pressed: true,
        }],
    );
    s.frame(
        &mut g,
        &[Raw::Key {
            code: KEY_SPACE,
            pressed: false,
        }],
    );
    assert_eq!(g.last_pass(), Pass::Whiffed);
    assert_eq!(
        g.score_pts(),
        (0, 0),
        "a whiff scores nothing but hurts nothing"
    );
    assert!(!g.clean_round(r), "a whiffed pass is no longer clean");
    // the foe lunges again on the next exchange - take it properly
    for _ in 0..120 {
        if g.exchange() {
            break;
        }
        s.frame(&mut g, &[]);
    }
    assert!(g.exchange(), "the whiffed bout moved on");
    s.win_pass(&mut g);
    assert!(g.won_round(r), "the round is still winnable after a whiff");
    assert!(!g.clean_round(r));
    assert_eq!(g.score_pts(), (1, 0));
}

#[test]
fn the_gold_flash_outlives_a_feint_and_the_decoy_guard_bleeds() {
    // find a seed whose third round feints
    let seed = (1u64..)
        .find(|s| plan(*s)[2].feint.is_some())
        .expect("a feint seed");
    let atk = plan(seed)[2];
    let mut s = Sim::new();
    let mut g = Sword::new(seed);
    s.wait_present(&mut g);
    // walk to round three without engaging: eat the first two passes
    while g.round_no() < 2 && !g.done() {
        let r = g.round_no();
        s.wait_round(&mut g, r);
    }
    assert_eq!(g.round_no(), 2);
    // the blade shows the lie at first...
    assert_eq!(g.pose(), atk.tell);
    // ...but the honest flash is already lit and the dip follows it
    let mut saw_flash = false;
    while g.exchange() {
        if g.feint_tell() {
            saw_flash = true;
            assert_eq!(g.pose(), atk.tell, "the flash must precede the dip");
        }
        s.frame(&mut g, &[]);
        if g.pose() == atk.final_lane() {
            break;
        }
    }
    assert!(saw_flash, "a feint without its tell is a coin flip");
    assert_eq!(g.pose(), atk.final_lane(), "the blade commits to the truth");
    // biting the decoy is a flinch - the blade lands
    s.parry(&mut g, atk.tell);
    while g.exchange() {
        s.frame(&mut g, &[]);
    }
    assert_eq!(g.last_pass(), Pass::Flinched);
    assert_eq!(g.score_pts().1, 3, "the decoy guard lost the bout");
}

#[test]
fn a_3_1_bout_lands_silver_not_gold() {
    let mut s = Sim::new();
    let mut g = Sword::new(3);
    s.wait_present(&mut g);
    // rounds 1 and 2: clean passes
    for _ in 0..2 {
        let r = g.round_no();
        s.win_pass(&mut g);
        s.wait_round(&mut g, r);
    }
    // round 3: stand open and give the pass away
    let r = g.round_no();
    s.wait_round(&mut g, r);
    assert_eq!(g.score_pts(), (2, 1));
    // round 4: close it out 3-1
    s.win_pass(&mut g);
    assert_eq!(g.score_pts(), (3, 1));
    while !g.done() {
        s.frame(&mut g, &[]);
    }
    assert_eq!(g.score(), 84, "clean passes minus the unhurt bonus");
    assert_eq!(g.state(), MgState::Won(Tier::Silver));
}

#[test]
fn a_run_of_whiffs_exhausts_the_round() {
    let mut s = Sim::new();
    let mut g = Sword::new(17);
    s.wait_present(&mut g);
    let r = g.round_no();
    for _ in 0..EXCH_MAX {
        for _ in 0..200 {
            if g.exchange() {
                break;
            }
            s.frame(&mut g, &[]);
        }
        if !g.exchange() {
            break; // round already closed
        }
        let l = g.lane_now();
        s.parry(&mut g, l);
        s.wait_stagger(&mut g);
        s.frame(
            &mut g,
            &[Raw::Key {
                code: KEY_SPACE,
                pressed: true,
            }],
        );
        s.frame(
            &mut g,
            &[Raw::Key {
                code: KEY_SPACE,
                pressed: false,
            }],
        );
        assert_eq!(g.last_pass(), Pass::Whiffed);
    }
    // let the last whiff's recovery beat call the round
    for _ in 0..80 {
        s.frame(&mut g, &[]);
    }
    assert_eq!(
        g.last_pass(),
        Pass::Even,
        "six whiffed exchanges call it even"
    );
    assert_eq!(g.score_pts(), (0, 0));
    assert!(g.done() || g.round_no() > r, "even closes the round");
    assert!(g.unhurt());
}

#[test]
fn factory_arms_sword_too() {
    use defender_of_the_crown::ui::host::make_game;
    let g = make_game("sword", 42).expect("sword is armed");
    assert_eq!(g.id(), "sword");
    assert!(
        make_game("archery", 1).is_none(),
        "T601 sports stay unarmed"
    );
}
