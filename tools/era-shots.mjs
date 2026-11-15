// T706: era pacing screenshots. Boots the real campaign at the era
// boundaries (turn 3 / 7 / 11 = one turn inside each unlock tier), opens
// the own-castle panel and verifies the SPORTS button paints the season's
// sport label pixel-exact against the host-side glyph oracle, then shots
// each era for the record.
// Parent run (no QA_TURN) spawns one child per turn and aggregates.
import { coords, shot, tap, px, finish } from "./jsdom-harness.mjs";

const TURNS = [3, 7, 11];
const cargo = `${process.env.HOME}/.cargo/bin/cargo`;

if (!process.env.QA_TURN) {
  const oracle = Bun.spawnSync([cargo, "run", "-q", "--example", "sports", ...TURNS.map(String)], {
    cwd: new URL("..", import.meta.url).pathname,
    stdout: "pipe",
  });
  if (oracle.exitCode !== 0) throw new Error(`oracle failed: ${await new Response(oracle.stderr).text()}`);
  const expected = {};
  let cur = null;
  for (const line of (await new Response(oracle.stdout).text()).split("\n")) {
    if (line.startsWith("@")) cur = { t: +line.slice(1).split(" ")[0], id: line.split(" ")[1], label: line.split(" ")[2], rows: [] };
    else if (cur && line) {
      cur.rows.push(line);
      if (cur.rows.length === 8) {
        expected[cur.t] = cur;
        cur = null;
      }
    }
  }
  let fails = 0;
  for (const t of TURNS) {
    const child = Bun.spawnSync(["bun", import.meta.path], {
      env: {
        ...process.env,
        QA_TURN: String(t),
        QA_LABEL: expected[t].label,
        QA_GRID: expected[t].rows.join("\n"),
        CROWN_QS: `scene=play&seed=7&turn=${t}&mute=1`,
      },
      stdout: "pipe",
      stderr: "inherit",
    });
    const out = (await new Response(child.stdout).text()).trim();
    const line = out.split("\n").find((l) => l.startsWith("{"));
    if (!line) {
      console.error(`turn ${t}: no result from child`);
      fails++;
      continue;
    }
    const r = JSON.parse(line);
    const good = r.match === true;
    console.log(
      `${good ? "pass" : "FAIL"}  turn ${t}: button shows ${expected[t].label} pixel-exact (${r.lit} lit px)`,
    );
    if (!good) {
      fails++;
      console.error(`  diffs=${r.diffs}\n  got:\n${r.gotRows?.join("\n")}`);
    }
  }
  const labels = TURNS.map((t) => `${expected[t].id}=${expected[t].label}`);
  const distinct = new Set(labels).size === labels.length;
  console.log(`${distinct ? "pass" : "FAIL"}  era labels differ per tier: ${labels.join(", ")}`);
  if (fails || !distinct) console.log("FAILS");
  else console.log("DONE clean");
  process.exit(fails || !distinct ? 1 : 0);
}

// ---- child: one era, one campaign boot
const t = +process.env.QA_TURN;
await Bun.sleep(800);
tap(...coords.BUCK); // own castle -> action panel
await Bun.sleep(250);
const label = process.env.QA_LABEL;
const x0 = 150 + (84 - label.length * 8) / 2;
const gotRows = [];
let lit = 0;
for (let row = 0; row < 8; row++) {
  let s = "";
  for (let x = x0; x < x0 + label.length * 8; x++) {
    const p = px(x, 162 + row);
    const on = p[0] + p[1] + p[2] > 150;
    if (on) lit++;
    s += on ? "#" : ".";
  }
  gotRows.push(s);
}
const want = process.env.QA_GRID.split("\n");
let diffs = 0;
for (let row = 0; row < 8; row++) for (let i = 0; i < want[row].length; i++) if (want[row][i] !== gotRows[row][i]) diffs++;
shot(`era-t${t}`);
console.log(JSON.stringify({ turn: t, label, match: diffs === 0 && lit > 20, lit, diffs, gotRows }));
finish();
