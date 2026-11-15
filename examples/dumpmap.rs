// Throwaway: dump the campaign map + dressed minigame frames as PPMs.
use defender_of_the_crown::campaign::game::Difficulty;
use defender_of_the_crown::engine::gfx::{Framebuffer, H, PALETTE, W};
use defender_of_the_crown::engine::input::Input;
use defender_of_the_crown::minigames::framework::Minigame;
use defender_of_the_crown::ui::CampaignUi;

fn dump(fb: &Framebuffer, path: &str) {
    let mut out = format!("P6\n{W} {H}\n255\n").into_bytes();
    for &i in &fb.data {
        let [r, g, b] = PALETTE[i as usize];
        out.extend_from_slice(&[r, g, b]);
    }
    std::fs::write(path, out).unwrap();
    println!("wrote {path}");
}

fn mg(mut g: impl Minigame, ticks: usize, path: &str) {
    let mut fb = Framebuffer::new();
    let inp = Input::new();
    for _ in 0..ticks {
        g.tick(1.0 / 60.0, &inp, &mut fb);
    }
    dump(&fb, path);
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/crown-dbg".into());
    let mut fb = Framebuffer::new();
    let mut u = CampaignUi::new(1, Difficulty::Normal);
    u.sel = Some(4);
    u.draw(&mut fb);
    dump(&fb, &format!("{dir}/map-relief.ppm"));
    mg(
        defender_of_the_crown::minigames::joust::Joust::new(1),
        40,
        &format!("{dir}/mg-joust.ppm"),
    );
    mg(
        defender_of_the_crown::minigames::falcon::Falcon::new(1),
        40,
        &format!("{dir}/mg-falcon.ppm"),
    );
    mg(
        defender_of_the_crown::minigames::sword::Sword::new(1),
        40,
        &format!("{dir}/mg-sword.ppm"),
    );
    mg(
        defender_of_the_crown::minigames::catapult::Catapult::new(1),
        40,
        &format!("{dir}/mg-catapult.ppm"),
    );
    mg(
        defender_of_the_crown::minigames::archery::Archery::new(1),
        40,
        &format!("{dir}/mg-archery.ppm"),
    );
}
