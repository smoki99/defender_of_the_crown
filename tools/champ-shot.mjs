// T707: champion triple-gold flow in the real campaign. Boots with
// champ=1 (banked champion + proclamation), verifies the modal pixels,
// dismisses it with OK, opens Buckingham and verifies the CHAMP X1 row
// pixel-exact against the label oracle, then shots both stages.
const cargo = `${process.env.HOME}/.cargo/bin/cargo`;
process.env.CROWN_QS = "scene=play&seed=7&champ=1&mute=1";

const oracle = Bun.spawnSync([cargo, "run", "-q", "--example", "labelgrid", "CHAMP X1"], {
  cwd: new URL("..", import.meta.url).pathname,
  stdout: "pipe",
});
if (oracle.exitCode !== 0) throw new Error(`oracle failed: ${await new Response(oracle.stderr).text()}`);
const grid = (await new Response(oracle.stdout).text()).split("\n").filter((l) => l && !l.startsWith("@"));
if (grid.length !== 8) throw new Error(`bad oracle grid: ${grid.length}`);

const { coords, shot, tap, px, finish } = await import("./jsdom-harness.mjs");
let fails = 0;
const check = (name, ok, extra = "") => {
  console.log(`${ok ? "pass" : "FAIL"}  ${name}${extra ? ` (${extra})` : ""}`);
  if (!ok) fails++;
};

await Bun.sleep(800);
// proclamation modal: LINEN plate, NAVY plaque, gold crown crest + jewel
const linen = (x, y) => {
  const [r, g, b] = px(x, y);
  return r > 110 && g > 110 && b > 70 && b < g + 60 && Math.abs(r - g) < 40;
};
check("modal: linen plate at (74,70)", linen(74, 70));
check("modal: linen plate at (240,130)", linen(240, 130));
const [gr, gg, gb] = px(83, 80);
check("modal: gold crown band", gr > 110 && gg > 80 && gb < 70, `${gr},${gg},${gb}`);
const [jr, jg, jb] = px(87, 80);
check("modal: crown jewel DARKRED", jr > 60 && jr < 150 && jg < 40 && jb < 40, `${jr},${jg},${jb}`);
shot("champ-modal");
// the map is deaf behind the proclamation: a castle tap selects nothing
tap(...coords.BUCK);
await Bun.sleep(120);
check("modal: castle tap deaf (plate stays)", linen(74, 70));
// only OK dismisses
tap(160, 128);
await Bun.sleep(150);
check("modal: OK clears the plate", !linen(74, 70));
// the champion rides with Buckingham's host
tap(...coords.BUCK);
await Bun.sleep(200);
let champ = -1;
const rows = [];
for (let i = 0; i < 4 && champ < 0; i++) {
  const y = 100 + i * 9;
  const got = [];
  for (let row = 0; row < 8; row++) {
    let s = "";
    for (let x = 10; x < 10 + 8 * 8; x++) {
      const p = px(x, y + row);
      s += p[0] + p[1] + p[2] > 300 ? "#" : ".";
    }
    got.push(s);
  }
  rows.push(got);
  if (got.every((r, ri) => r === grid[ri])) champ = i;
}
check(`panel: CHAMP X1 row pixel-exact (row ${champ})`, champ >= 0, champ < 0 ? `got: ${rows[0]?.[0]} / want ${grid[0]}` : "");
shot("champ-panel");
console.log(fails ? "FAILS" : "DONE clean");
finish();
process.exit(fails ? 1 : 0);
