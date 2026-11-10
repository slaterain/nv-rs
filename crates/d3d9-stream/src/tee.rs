//! Two backends fed the same commands (`NV_D3D9=real,wgpu`): the primary draws for the player and
//! answers; the shadow gets every command after it, so the two can be compared within one run
//! (shots of the same `frame`).

use crate::cmd::{Backend, Cmd, Data, Reply};

/// A backend that is two: the primary's replies are the answers; the shadow's are dropped. A
/// shadow that panics is printed once and left out from then on; the primary goes on.
///
/// The shadow gets the same window handles as the primary: it should draw off screen.
///
/// Where both answer a query's result (`GetData` done on both, `S_OK`), a shadow whose count
/// differs is printed (the first 20 times), and the first agreements too, so a run shows the
/// comparison was made: a diagnostic, the primary's answer stands.
pub struct Tee {
    primary: Box<dyn Backend>,
    shadow: Option<Box<dyn Backend>>,
    queries: (u64, u64),
}

impl Tee {
    pub fn new(primary: Box<dyn Backend>, shadow: Box<dyn Backend>) -> Tee {
        Tee {
            primary,
            shadow: Some(shadow),
            queries: (0, 0),
        }
    }

    /// Whether the shadow still gets the commands.
    pub fn shadow_running(&self) -> bool {
        self.shadow.is_some()
    }

    /// How many query results both answered: (the same, different).
    pub fn query_results(&self) -> (u64, u64) {
        self.queries
    }

    fn compare(&mut self, cmd: &Cmd<Data<'_>>, primary: &Reply, shadow: &Reply) {
        let (
            Cmd::QueryGetData { query, .. },
            Reply::Query(Some((0, a))),
            Reply::Query(Some((0, b))),
        ) = (cmd, primary, shadow)
        else {
            return;
        };
        if a == b {
            self.queries.0 += 1;
            if self.queries.0 <= 3 {
                eprintln!(
                    "NV_D3D9 tee: query {query:#x}: the shadow's result is the primary's ({a})"
                );
            }
        } else {
            self.queries.1 += 1;
            if self.queries.1 <= 20 {
                eprintln!(
                    "NV_D3D9 tee: query {query:#x}: the primary's result {a}, the shadow's {b}"
                );
            }
        }
    }
}

impl Backend for Tee {
    fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
        let reply = self.primary.execute(cmd);
        if let Some(shadow) = &mut self.shadow {
            let ran =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| shadow.execute(cmd)));
            match ran {
                Ok(theirs) => self.compare(&cmd, &reply, &theirs),
                Err(panic) => {
                    eprintln!(
                        "the shadow backend stopped at {}: {}; the primary goes on alone",
                        cmd.name(),
                        crate::runner::panic_message(&*panic)
                    );
                    // Dropped inside a guard too: a shadow left broken may panic in its drop.
                    let failed = self.shadow.take();
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                        drop(failed);
                    }));
                }
            }
        }
        reply
    }
}
