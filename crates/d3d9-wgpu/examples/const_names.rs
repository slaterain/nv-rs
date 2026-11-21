//! A measuring tool: `const_names FILE FROM TO`. For the draws of
//! Presents [FROM, TO), the name the bound shader's constant table gives each float register it uses, counted:
//! per register, its names with how many draws a frame read it under that name (vertex `vs`, pixel `ps`).
use d3d9_stream::Cmd;
use std::collections::{BTreeMap, HashMap};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let bytes = std::fs::read(&a[0]).expect("read");
    let (from, to): (u64, u64) = (a[1].parse().unwrap(), a[2].parse().unwrap());
    let mut tables: HashMap<u32, Vec<(u16, u16, String)>> = HashMap::new();
    let mut bound = [0u32; 2];
    let mut present = 0u64;
    let mut names: BTreeMap<(u8, u16), BTreeMap<String, u64>> = BTreeMap::new();
    for c in d3d9_stream::wire::commands(&bytes).expect("stream") {
        let c = c.expect("command");
        match &c {
            Cmd::CreateShader { shader, tokens, .. } => {
                if let Ok(p) = d3d9_wgpu::sm::parse(&tokens.to_words()) {
                    tables.insert(
                        *shader,
                        p.constants
                            .iter()
                            .filter(|k| k.set == 2)
                            .map(|k| (k.index, k.count, k.name.clone()))
                            .collect(),
                    );
                }
            }
            Cmd::SetShader { pixel, shader, .. } => bound[usize::from(*pixel)] = *shader,
            Cmd::DrawIndexedPrimitive { .. } | Cmd::DrawPrimitive { .. }
                if present >= from && present < to =>
            {
                for (stage, h) in bound.iter().enumerate() {
                    for (index, count, name) in tables.get(h).map_or(&[][..], |v| v.as_slice()) {
                        for r in *index..index + count {
                            *names
                                .entry((stage as u8, r))
                                .or_default()
                                .entry(name.clone())
                                .or_default() += 1;
                        }
                    }
                }
            }
            Cmd::Present { .. } => present += 1,
            _ => {}
        }
    }
    let frames = (to - from).max(1) as f64;
    for ((stage, r), m) in &names {
        let mut v: Vec<_> = m.iter().collect();
        v.sort_by(|x, y| y.1.cmp(x.1));
        let s: Vec<String> = v
            .iter()
            .take(4)
            .map(|(n, c)| format!("{n} {:.0}", **c as f64 / frames))
            .collect();
        println!(
            "{} c{r}: {}",
            if *stage == 0 { "vs" } else { "ps" },
            s.join(", ")
        );
    }
}
