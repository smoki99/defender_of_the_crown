// T501 browser smoke: the SPORTS button on a Saxon castle answers the
// player - meet-not-held or tourney-ground message in the ticker, no crash.
import { coords, tap, key, px, snap, regionDiff, shot, ok, finish, problems } from "./jsdom-harness.mjs";

await Bun.sleep(700);
key(" "); // boot herald
await Bun.sleep(120);
key("Enter"); // BEGIN CAMPAIGN
await Bun.sleep(120);
key("ArrowDown"); // NORMAL
key("Enter");
await Bun.sleep(900);
key("Enter"); key("Enter"); key("Enter"); // crawl -> campaign
await Bun.sleep(900);

const [bx, by] = coords.BUCK;
tap(bx, by); // open the own-castle roll (panel has the SPORTS button)
await Bun.sleep(150);
const before = snap();

tap(...coords.SPORTS);
await Bun.sleep(200);
const after = snap();
const ticker = regionDiff(before, after, 0, 244, 320, 256);
shot("sports");
ok(ticker > 40, `SPORTS tap printed an answer in the ticker (${ticker}px)`);
// the panel must still be open (the message is not a scene switch)
const panel = regionDiff(after, snap(), 0, 88, 216, 132);
ok(panel < 30, "stay on the campaign map after the SPORTS tap");
ok(problems.length === 0, `no errors (${problems.slice(0, 3).join(" | ") || "clean"})`);
await finish();
