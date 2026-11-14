use crate::campaign::ai;
use crate::campaign::combat::{self, BattleOutcome, FULL_WALLS};
use crate::campaign::map;
use crate::campaign::rng::Xor;
use crate::campaign::units::{KindCounts, Unit, UnitKind, MAX_GARRISON};
use crate::campaign::events::EventKind;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Faction {
    Saxon,
    Norman,
    /// Barons who rose from zero loyalty (SPEC 5.2); own land, never act.
    Neutral,
}

impl Faction {
    pub fn other(self) -> Faction {
        match self {
            Faction::Saxon => Faction::Norman,
            Faction::Norman => Faction::Saxon,
            Faction::Neutral => Faction::Neutral,
        }
    }
    pub fn index(self) -> usize {
        match self {
            Faction::Saxon => 0,
            Faction::Norman => 1,
            Faction::Neutral => 2,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Faction::Saxon => "Saxons",
            Faction::Norman => "Normans",
            Faction::Neutral => "Rebels",
        }
    }
}

pub const PLAYER: Faction = Faction::Saxon;

/// Kingdom meters besides gold (SPEC 5.2), clamped per field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bars {
    /// Eaten 1/turn per soldier; short armies desert and loyalty falls.
    pub provisions: u32,
    /// 0..=100. Scales income; at 0 castles revolt to Neutral each turn.
    pub loyalty: u32,
    /// 0..=100. Halves recruit prices at 100 (SPEC 5.2).
    pub popularity: u32,
}

pub const BAR_MAX: u32 = 100;

pub fn starting_bars() -> Bars {
    Bars {
        provisions: 60,
        loyalty: 70,
        popularity: 60,
    }
}

#[derive(Clone, Debug)]
pub struct Stronghold {
    pub id: usize,
    pub name: &'static str,
    pub owner: Faction,
    pub capital: bool,
    pub income: u32,
    pub coast: bool,
    pub port: bool,
    pub x: f32,
    pub y: f32,
    pub adj: Vec<usize>,
    pub garrison: Vec<Unit>,
    pub walls: u32,
    pub moved: bool,
}

impl Stronghold {
    pub fn size(&self) -> usize {
        self.garrison.len()
    }
}

/// What one faction last learned about a stronghold (SPEC 5.3 fog).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Intel {
    pub counts: crate::campaign::units::KindCounts,
    pub walls: u32,
    /// Turn the observation was made; stale reports keep their stamp.
    pub seen: u32,
}

/// What a faction knows about one stronghold (SPEC 5.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Own(Intel),
    /// Scouts eyeball "SEEN: ~N": within ±50% of the truth.
    Rough { total: u32 },
    /// Exact but stamped spy report.
    Spied(Intel),
    /// No eyes, no report.
    Dark,
}

pub const SPY_COST: u32 = 15;

#[derive(Clone, Debug)]
pub struct GameState {
    pub turn: u32,
    pub rng: Xor,
    /// Events and spy whispers draw here, never off the battle stream.
    pub event_rng: Xor,
    salt: u64,
    /// Next-turn event leaked by spies; consumed when events land (T206).
    pub tip: Option<EventKind>,
    /// [Saxon, Norman, Neutral-unused]
    pub gold: [u32; 3],
    /// Indexed like `gold`; Neutral never holds bars.
    pub bars: [Bars; 3],
    pub strongholds: Vec<Stronghold>,
    /// Per castle: [Saxon, Norman] last spy report (fog of war).
    pub known: Vec<[Option<Intel>; 2]>,
    pub log: Vec<String>,
    pub over: Option<Faction>,
}

impl GameState {
    pub fn new(seed: u64) -> Self {
        let strongholds: Vec<Stronghold> = map::defs()
            .into_iter()
            .enumerate()
            .map(|(id, d)| Stronghold {
                id,
                name: d.name,
                owner: if d.is_saxon { Faction::Saxon } else { Faction::Norman },
                capital: d.capital,
                income: d.income,
                x: d.x,
                y: d.y,
                coast: d.coast,
                port: d.port,
                adj: d.adj.to_vec(),
                garrison: if d.capital {
                    starting_capital()
                } else {
                    starting_garrison()
                },
                walls: FULL_WALLS,
                moved: false,
            })
            .collect();
        let n = strongholds.len();
        let mut g = GameState {
            turn: 1,
            rng: Xor::new(seed),
            event_rng: Xor::new(seed ^ 0xE6E71),
            salt: seed ^ 0x5A175,
            tip: None,
            gold: [70, 70, 0],
            bars: [starting_bars(); 3],
            strongholds,
            known: vec![[None, None]; n],
            log: Vec::new(),
            over: None,
        };
        g.push_log("1066. THE NORMANS HAVE LANDED.");
        g.push_log("RECLAIM ENGLAND, LORD OF THE SAXONS!");
        g
    }

    pub fn gi(&self, f: Faction) -> u32 {
        self.gold[f.index()]
    }

    pub fn bars_of(&self, f: Faction) -> Bars {
        self.bars[f.index()]
    }

    /// True own eyes / adjacent scouts (SPEC 5.3).
    pub fn can_see(&self, f: Faction, sid: usize) -> bool {
        let s = &self.strongholds[sid];
        s.owner == f || s.adj.iter().any(|&a| self.strongholds[a].owner == f)
    }

    fn snapshot(&self, sid: usize) -> Intel {
        Intel {
            counts: self.counts(sid),
            walls: self.strongholds[sid].walls,
            seen: self.turn,
        }
    }

    /// What `f` can act on (SPEC 5.3 fog).
    pub fn view(&self, f: Faction, sid: usize) -> View {
        let slot = f.index().min(1);
        if self.strongholds[sid].owner == f {
            View::Own(self.snapshot(sid))
        } else if let Some(rep) = self.known[sid][slot] {
            View::Spied(rep)
        } else if self.can_see(f, sid) {
            View::Rough {
                total: self.estimate(sid),
            }
        } else {
            View::Dark
        }
    }

    /// Stable "SEEN: ~N" guess inside ±50% of truth; same inputs, same lie.
    fn estimate(&self, sid: usize) -> u32 {
        let total = self.strongholds[sid].garrison.len() as u64;
        let mut z = self
            .salt
            .wrapping_mul(sid as u64 + 0x9E3779B97F4A7C15)
            .wrapping_add(total.wrapping_mul(0xBF58476D1CE4E5B9));
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^= z >> 31;
        (total / 2 + z % (total + 1)) as u32
    }

    /// SPEC 5.3: 70% to reveal castle + neighbours, failure still costs the
    /// gold (repeatable); one time in ten a servant leaks tomorrow's event.
    pub fn spy(&mut self, f: Faction, sid: usize) -> Result<bool, String> {
        if self.over.is_some() {
            return Err("THE GAME IS OVER.".into());
        }
        if self.strongholds[sid].owner == f {
            return Err("NO NEED TO SPY ON YOUR OWN.".into());
        }
        if self.gi(f) < SPY_COST {
            return Err("NOT ENOUGH GOLD.".into());
        }
        self.gold[f.index()] -= SPY_COST;
        let name = self.strongholds[sid].name;
        let slot = f.index().min(1);
        if self.rng.below(100) < 70 {
            let ring = self.strongholds[sid].adj.clone();
            let snap = self.snapshot(sid);
            self.known[sid][slot] = Some(snap);
            for a in ring {
                let s = self.snapshot(a);
                self.known[a][slot] = Some(s);
            }
            self.push_log(format!("SPIES REPORT FROM {name}."));
            if self.rng.below(100) < 10 {
                self.tip = Some(EventKind::roll(&mut self.event_rng, self.turn + 1));
                self.push_log("A SERVANT MURMURS OF TOMORROW.");
            }
            Ok(true)
        } else {
            self.push_log(format!("OUR SPY WAS CAUGHT AT {name}."));
            Ok(false)
        }
    }

    fn add_bar(&mut self, f: Faction, which: u8, delta: i32) {
        let max = if which == 2 {
            self.provisions_cap(f)
        } else {
            BAR_MAX
        };
        let b = &mut self.bars[f.index()];
        let v = match which {
            0 => &mut b.loyalty,
            1 => &mut b.popularity,
            _ => &mut b.provisions,
        };
        *v = (*v as i64 + delta as i64).clamp(0, max as i64) as u32;
    }

    pub fn add_loyalty(&mut self, f: Faction, delta: i32) {
        self.add_bar(f, 0, delta);
    }

    pub fn add_popularity(&mut self, f: Faction, delta: i32) {
        self.add_bar(f, 1, delta);
    }

    pub fn add_provisions(&mut self, f: Faction, delta: i32) {
        self.add_bar(f, 2, delta);
    }

    /// Market purchase: 2 gold per provision, clamped to the barn cap.
    pub fn buy_provisions(&mut self, f: Faction, want: u32) -> Result<(), String> {
        if self.over.is_some() {
            return Err("THE GAME IS OVER.".into());
        }
        let cap = self.provisions_cap(f);
        let room = cap - self.bars[f.index()].provisions.min(cap);
        let n = want.min(room);
        let cost = 2 * n;
        if self.gi(f) < cost {
            return Err("NOT ENOUGH GOLD.".into());
        }
        self.gold[f.index()] -= cost;
        self.add_provisions(f, n as i32);
        Ok(())
    }

    pub fn troops_of(&self, f: Faction) -> usize {
        self.strongholds
            .iter()
            .filter(|s| s.owner == f)
            .map(|s| s.size())
            .sum()
    }

    pub fn provisions_cap(&self, f: Faction) -> u32 {
        10 * self.ids_of(f).len().max(1) as u32
    }

    /// Price after the popularity discount (SPEC 5.2: half at 100).
    pub fn effective_cost(&self, f: Faction, kind: UnitKind) -> u32 {
        let pop = self.bars[f.index()].popularity;
        kind.cost() * (2 * BAR_MAX - pop) / (2 * BAR_MAX)
    }

    /// Payday: scaled income, provisions eaten per soldier, hunger on shortfall.
    fn pay_day(&mut self, f: Faction) {
        let idx = f.index();
        let inc = self.income_of(f);
        self.gold[idx] += inc;
        let troops = self.troops_of(f) as i64;
        let stock = self.bars[idx].provisions.min(self.provisions_cap(f)) as i64;
        if troops > stock {
            self.bars[idx].provisions = 0;
            self.starve(f);
            self.add_loyalty(f, -3);
        } else {
            self.bars[idx].provisions = (stock - troops) as u32;
        }
    }

    /// One unfed turn: the biggest army wastes away a fifth, levies first.
    fn starve(&mut self, f: Faction) {
        let target = self
            .strongholds
            .iter()
            .filter(|s| s.owner == f)
            .max_by_key(|s| (s.garrison.len(), std::cmp::Reverse(s.id)))
            .map(|s| s.id);
        let Some(id) = target else { return };
        let n = self.strongholds[id].garrison.len().div_ceil(5);
        if n == 0 {
            return;
        }
        let g = &mut self.strongholds[id].garrison;
        g.sort_by_key(|u| u.kind.index());
        let lost = g.drain(..n).count();
        let name = self.strongholds[id].name;
        self.push_log(format!("HUNGER TAKES {lost} IN {name}."));
    }

    /// At zero loyalty one castle a turn joins the rebels (SPEC 5.2).
    fn maybe_revolt(&mut self, f: Faction) {
        if self.bars[f.index()].loyalty != 0 {
            return;
        }
        let pick = self
            .strongholds
            .iter()
            .filter(|s| s.owner == f && !s.capital)
            .min_by_key(|s| s.id)
            .map(|s| s.id);
        if let Some(id) = pick {
            let s = &mut self.strongholds[id];
            s.owner = Faction::Neutral;
            s.garrison.clear();
            s.walls = FULL_WALLS;
            let name = s.name;
            self.push_log(format!("{name} REVOLTS! THE BARONS RISE."));
        }
    }

    /// Harvest scaled by the loyalty bar: half at 0, half again at 100.
    pub fn income_of(&self, f: Faction) -> u32 {
        let raw: u32 = self
            .strongholds
            .iter()
            .filter(|s| s.owner == f)
            .map(|s| s.income)
            .sum();
        raw * (BAR_MAX / 2 + self.bars[f.index()].loyalty) / BAR_MAX
    }

    pub fn ids_of(&self, f: Faction) -> Vec<usize> {
        (0..self.strongholds.len())
            .filter(|&i| self.strongholds[i].owner == f)
            .collect()
    }

    pub fn adj(&self, sid: usize) -> Vec<usize> {
        self.strongholds[sid].adj.clone()
    }

    pub fn counts(&self, sid: usize) -> KindCounts {
        combat::counts(&self.strongholds[sid].garrison)
    }

    pub fn army_power(&self, sid: usize) -> u32 {
        self.strongholds[sid].garrison.iter().map(|u| u.power()).sum()
    }

    pub fn defense_power(&self, sid: usize) -> u32 {
        let s = &self.strongholds[sid];
        let garrison_power: u32 = s.garrison.iter().map(|u| u.power()).sum();
        let siege_present = s.garrison.iter().any(|u| u.kind == UnitKind::Siege);
        let walls = if siege_present { s.walls / 3 } else { s.walls };
        garrison_power + walls
    }

    fn push_log(&mut self, line: impl Into<String>) {
        self.log.push(line.into());
        if self.log.len() > 40 {
            self.log.remove(0);
        }
    }

    // ---------------------------------------------------------------- actions

    pub fn can_recruit(&self, f: Faction, sid: usize, kind: UnitKind) -> Result<(), String> {
        if self.over.is_some() {
            return Err("THE GAME IS OVER.".into());
        }
        let s = &self.strongholds[sid];
        if s.owner != f {
            return Err("NOT YOUR STRONGHOLD.".into());
        }
        if kind.special() {
            return Err("CHAMPIONS ARE EARNED, NOT BOUGHT.".into());
        }
        if s.size() >= MAX_GARRISON {
            return Err("GARRISON FULL (40).".into());
        }
        let cost = self.effective_cost(f, kind);
        if self.gi(f) < cost {
            return Err("NOT ENOUGH GOLD.".into());
        }
        Ok(())
    }

    pub fn recruit(&mut self, f: Faction, sid: usize, kind: UnitKind) -> Result<(), String> {
        self.can_recruit(f, sid, kind)?;
        let cost = self.effective_cost(f, kind);
        self.gold[f.index()] -= cost;
        self.strongholds[sid].garrison.push(Unit::new(kind));
        Ok(())
    }

    pub fn can_march(&self, f: Faction, from: usize, to: usize) -> Result<(), String> {
        if self.over.is_some() {
            return Err("THE GAME IS OVER.".into());
        }
        let s = &self.strongholds[from];
        if s.owner != f {
            return Err("NOT YOUR STRONGHOLD.".into());
        }
        if s.moved {
            return Err("ALREADY MARCHED THIS TURN.".into());
        }
        if s.garrison.is_empty() {
            return Err("NO TROOPS TO MARCH.".into());
        }
        if !s.adj.contains(&to) || to == from {
            return Err("NOT ADJACENT.".into());
        }
        Ok(())
    }

    pub fn march(&mut self, f: Faction, from: usize, to: usize) -> Result<(), String> {
        self.can_march(f, from, to)?;
        let army = std::mem::take(&mut self.strongholds[from].garrison);
        self.strongholds[from].moved = true;

        if self.strongholds[to].owner == f {
            let n = army.len();
            self.strongholds[to].garrison.extend(army);
            // no two-leg day: the reinforced castle also spent its march
            self.strongholds[to].moved = true;
            let name = self.strongholds[to].name;
            self.push_log(format!("{n} TROOPS MARCH TO {name}."));
        } else {
            let def_name = self.strongholds[to].name.to_string();
            let atk_name = self.strongholds[from].name.to_string();
            let def_f = self.strongholds[to].owner;
            let walls = self.strongholds[to].walls;
            let def = std::mem::take(&mut self.strongholds[to].garrison);
            let out = combat::simulate(&army, &def, walls, &mut self.rng);
            self.log_battle(&out, &atk_name, &def_name, f);
            if out.attacker_won {
                self.strongholds[to].owner = f;
                self.strongholds[to].garrison = out.atk_survivors.clone();
                self.strongholds[to].walls = FULL_WALLS;
                // the victorious army has spent its day's march
                self.strongholds[to].moved = true;
                self.push_log(format!("{} IS YOURS!", def_name));
                self.add_popularity(f, 2);
                if def_f != Faction::Neutral {
                    self.add_loyalty(def_f, -3);
                }
            } else {
                self.strongholds[to].garrison = out.def_survivors.clone();
                self.add_loyalty(f, -3);
                if def_f != Faction::Neutral {
                    self.add_popularity(def_f, 1);
                }
            }
            // a battle you fought is a battle you saw: fog lifts here
            self.known[to][f.index().min(1)] = Some(self.snapshot(to));
            self.check_over();
        }
        Ok(())
    }

    fn log_battle(&mut self, out: &BattleOutcome, _atk: &str, def: &str, f: Faction) {
        let verb = if f == PLAYER { "YOUR ATTACK ON" } else { "THE ATTACK ON" };
        self.push_log(format!(
            "{} {}: {} IN {} ROUNDS.",
            verb,
            def,
            if out.attacker_won { "WON" } else { "REPULSED" },
            out.rounds
        ));
        self.push_log(format!(
            "LOST {} | ENEMY LOST {}",
            combat::fmt_counts(&out.atk_loss),
            combat::fmt_counts(&out.def_loss)
        ));
    }

    fn check_over(&mut self) {
        if self.over.is_some() {
            return;
        }
        for f in [Faction::Saxon, Faction::Norman] {
            if self.ids_of(f).is_empty() {
                let w = f.other();
                self.over = Some(w);
                self.push_log(if w == PLAYER {
                    "ENGLAND IS FREE! VICTORY!"
                } else {
                    "ENGLAND IS LOST. DEFEAT."
                }
                    .to_string());
                return;
            }
        }
    }

    /// Player ends their turn: the Normans act, then the new round begins
    /// (payday with bars upkeep, revolts, wall repairs, orders reset).
    pub fn end_player_turn(&mut self) {
        if self.over.is_some() {
            return;
        }
        ai::take_turn(self, Faction::Norman);
        if self.over.is_some() {
            return;
        }
        self.turn += 1;
        for f in [Faction::Saxon, Faction::Norman] {
            self.pay_day(f);
        }
        for f in [Faction::Saxon, Faction::Norman] {
            self.maybe_revolt(f);
            self.check_over();
        }
        for s in self.strongholds.iter_mut() {
            s.walls = FULL_WALLS;
            s.moved = false;
        }
    }
}

fn starting_capital() -> Vec<Unit> {
    let mut v = Vec::new();
    for k in [UnitKind::Knight, UnitKind::Knight] {
        v.push(Unit::new(k));
    }
    for _ in 0..4 {
        v.push(Unit::new(UnitKind::Bowman));
    }
    for _ in 0..6 {
        v.push(Unit::new(UnitKind::Levy));
    }
    v
}

fn starting_garrison() -> Vec<Unit> {
    let mut v = Vec::new();
    v.push(Unit::new(UnitKind::Knight));
    for _ in 0..3 {
        v.push(Unit::new(UnitKind::Bowman));
    }
    for _ in 0..6 {
        v.push(Unit::new(UnitKind::Levy));
    }
    v
}
