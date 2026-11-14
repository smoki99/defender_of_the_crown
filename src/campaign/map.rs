/// The strategic map: 12 strongholds of 11th-century England, six Saxon and
/// six Norman, laid out for a 320x200 pixel scene (coordinates doubled to a
/// 640x400 logical field, scaled x3 on screen).
#[derive(Clone, Copy, Debug)]
pub struct StrongholdDef {
    pub name: &'static str,
    pub capital: bool,
    pub income: u32,
    pub x: f32,
    pub y: f32,
    pub is_saxon: bool,
    /// Raidable coast (Danish raiders, SPEC 5.4): Dover/London/Wallingford.
    pub coast: bool,
    /// Norman reinforcement port (choke, SPEC 5.1): Dover.
    pub port: bool,
    pub adj: &'static [usize],
}

/// Sanity-tested in tests: every edge is symmetric and the graph connected.
pub fn defs() -> Vec<StrongholdDef> {
    vec![
        // --- Saxon strongholds (the south-west) ---
        StrongholdDef { name: "Buckingham", capital: true, income: 5, x: 250., y: 200., is_saxon: true, coast: false, port: false, adj: &[1, 3, 5, 6, 10] },
        StrongholdDef { name: "Oxford",     capital: false, income: 4, x: 215., y: 160., is_saxon: true, coast: false, port: false, adj: &[0, 4, 10] },
        StrongholdDef { name: "Salisbury",  capital: false, income: 4, x: 130., y: 255., is_saxon: true, coast: false, port: false, adj: &[3, 4] },
        StrongholdDef { name: "Winchester", capital: false, income: 3, x: 195., y: 265., is_saxon: true, coast: false, port: false, adj: &[0, 2, 5] },
        StrongholdDef { name: "Devizes",    capital: false, income: 3, x: 85.,  y: 220., is_saxon: true, coast: false, port: false, adj: &[1, 2, 5] },
        StrongholdDef { name: "Wallingford",capital: false, income: 3, x: 235., y: 235., is_saxon: true, coast: true, port: false, adj: &[0, 3, 4] },
        // --- Norman strongholds (the east and north) ---
        StrongholdDef { name: "London",     capital: true, income: 5, x: 320., y: 180., is_saxon: false, coast: true, port: false, adj: &[0, 7, 10, 11] },
        StrongholdDef { name: "Colchester", capital: false, income: 4, x: 390., y: 145., is_saxon: false, coast: false, port: false, adj: &[6, 8] },
        StrongholdDef { name: "Norwich",    capital: false, income: 3, x: 415., y: 90.,  is_saxon: false, coast: false, port: false, adj: &[7, 9] },
        StrongholdDef { name: "York",       capital: false, income: 4, x: 350., y: 35.,  is_saxon: false, coast: false, port: false, adj: &[8, 10] },
        StrongholdDef { name: "Lincoln",    capital: false, income: 3, x: 280., y: 90.,  is_saxon: false, coast: false, port: false, adj: &[0, 1, 6, 9] },
        StrongholdDef { name: "Dover",      capital: false, income: 3, x: 410., y: 215., is_saxon: false, coast: true, port: true, adj: &[6] },
    ]
}

/// The road network as unique `a < b` edges — what the renderer draws.
pub fn roads() -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (i, d) in defs().iter().enumerate() {
        for &j in d.adj {
            if i < j {
                out.push((i, j));
            }
        }
    }
    out
}
