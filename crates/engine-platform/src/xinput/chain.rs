//! The game's four users over several kinds of pad: the system's XInput pads first, then the
//! other lists (the PlayStation pads read through HID on Windows, the pads of Linux's event
//! devices).

use super::{PadSource, ERROR_DEVICE_NOT_CONNECTED, ERROR_SUCCESS, USERS};

/// No XInput: the system has none (Linux), so a [`Chain`] of it is only its lists.
pub enum NoXInput {}

impl PadSource for NoXInput {
    fn get_state(&mut self, _: u32, _: &mut [u8; 16]) -> u32 {
        match *self {}
    }

    fn set_state(&mut self, _: u32, _: &[u8; 4]) -> u32 {
        match *self {}
    }
}

/// A list of pads of one kind: the `n`-th connected one, `n` from 0.
pub trait PadList: Send {
    /// How many are connected now.
    fn count(&mut self) -> usize;
    /// [`PadSource::get_state`] for the `n`-th.
    fn get_state(&mut self, n: usize, state: &mut [u8; 16]) -> u32;
    /// [`PadSource::set_state`] for the `n`-th, with XInput's left (strong) and right (weak) motor
    /// speeds.
    fn set_state(&mut self, n: usize, left: u16, right: u16) -> u32;
}

/// The game's users over the system's XInput pads and other lists: XInput's connected users first,
/// in their order, then each list's pads. With one pad, whichever kind, it is user 0, the one the
/// game reads. A pad that shows up twice (a tool that makes a virtual Xbox pad of a PlayStation
/// one, or Steam's) is read through XInput, where its user comes first.
pub struct Chain<X> {
    xinput: Option<X>,
    lists: Vec<Box<dyn PadList>>,
    /// Where each user's last rumble went.
    rumbled: [Option<Place>; USERS as usize],
}

/// Where a user's pad is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Place {
    XInput(u32),
    List(usize, usize),
    None,
}

impl<X: PadSource> Chain<X> {
    pub fn new(xinput: Option<X>, lists: Vec<Box<dyn PadList>>) -> Chain<X> {
        Chain {
            xinput,
            lists,
            rumbled: [None; USERS as usize],
        }
    }

    /// The pad that is the game's `user`; `state` gets the XInput pad's state on the way (and is
    /// left as it was otherwise).
    fn find(&mut self, user: u32, state: &mut [u8; 16]) -> Place {
        let mut n = user as usize;
        if let Some(x) = self.xinput.as_mut() {
            for u in 0..USERS {
                let mut s = *state;
                if x.get_state(u, &mut s) == ERROR_SUCCESS {
                    if n == 0 {
                        *state = s;
                        return Place::XInput(u);
                    }
                    n -= 1;
                }
            }
        }
        for (i, list) in self.lists.iter_mut().enumerate() {
            let count = list.count();
            if n < count {
                return Place::List(i, n);
            }
            n -= count;
        }
        Place::None
    }
}

impl<X: PadSource> PadSource for Chain<X> {
    fn get_state(&mut self, user: u32, state: &mut [u8; 16]) -> u32 {
        match self.find(user, state) {
            Place::XInput(_) => ERROR_SUCCESS,
            Place::List(i, n) => self.lists[i].get_state(n, state),
            Place::None => ERROR_DEVICE_NOT_CONNECTED,
        }
    }

    fn set_state(&mut self, user: u32, vibration: &[u8; 4]) -> u32 {
        let mut scratch = [0u8; 16];
        let place = self.find(user, &mut scratch);
        // A list's pad that is no longer this user (an Xbox pad plugged in before it took the
        // place) has its motors stopped: the game's stop now goes to the other pad (review of
        // #783).
        if let Some(slot) = self.rumbled.get_mut(user as usize) {
            if let Some(Place::List(i, n)) = *slot {
                if place != Place::List(i, n) {
                    self.lists[i].set_state(n, 0, 0);
                }
            }
            *slot = Some(place);
        }
        match place {
            Place::XInput(u) => match self.xinput.as_mut() {
                Some(x) => x.set_state(u, vibration),
                None => ERROR_DEVICE_NOT_CONNECTED,
            },
            Place::List(i, n) => {
                let left = u16::from_le_bytes([vibration[0], vibration[1]]);
                let right = u16::from_le_bytes([vibration[2], vibration[3]]);
                self.lists[i].set_state(n, left, right)
            }
            Place::None => ERROR_DEVICE_NOT_CONNECTED,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xinput::PadState;
    use std::sync::{Arc, Mutex};

    /// XInput with the users in `connected` plugged in, each user's state its number in the
    /// packet; the vibrations asked for kept.
    struct FakeXInput {
        connected: Vec<u32>,
        set: Arc<Mutex<Vec<(u32, [u8; 4])>>>,
    }

    impl PadSource for FakeXInput {
        fn get_state(&mut self, user: u32, state: &mut [u8; 16]) -> u32 {
            if self.connected.contains(&user) {
                *state = PadState {
                    packet: 100 + user,
                    ..PadState::default()
                }
                .to_bytes();
                ERROR_SUCCESS
            } else {
                ERROR_DEVICE_NOT_CONNECTED
            }
        }
        fn set_state(&mut self, user: u32, vibration: &[u8; 4]) -> u32 {
            if self.connected.contains(&user) {
                self.set.lock().unwrap().push((user, *vibration));
                ERROR_SUCCESS
            } else {
                ERROR_DEVICE_NOT_CONNECTED
            }
        }
    }

    /// A list of `count` pads, the `n`-th with the packet `base + n`; the speeds asked for kept.
    struct FakeList {
        count: usize,
        base: u32,
        set: Arc<Mutex<Vec<(usize, u16, u16)>>>,
    }

    impl PadList for FakeList {
        fn count(&mut self) -> usize {
            self.count
        }
        fn get_state(&mut self, n: usize, state: &mut [u8; 16]) -> u32 {
            if n < self.count {
                *state = PadState {
                    packet: self.base + n as u32,
                    ..PadState::default()
                }
                .to_bytes();
                ERROR_SUCCESS
            } else {
                ERROR_DEVICE_NOT_CONNECTED
            }
        }
        fn set_state(&mut self, n: usize, left: u16, right: u16) -> u32 {
            if n < self.count {
                self.set.lock().unwrap().push((n, left, right));
                ERROR_SUCCESS
            } else {
                ERROR_DEVICE_NOT_CONNECTED
            }
        }
    }

    type Log<T> = Arc<Mutex<Vec<T>>>;

    fn chain(
        xinput: Option<Vec<u32>>,
        lists: &[(usize, u32)],
    ) -> (
        Chain<FakeXInput>,
        Log<(u32, [u8; 4])>,
        Vec<Log<(usize, u16, u16)>>,
    ) {
        let xset = Log::default();
        let x = xinput.map(|connected| FakeXInput {
            connected,
            set: xset.clone(),
        });
        let mut logs = Vec::new();
        let mut boxed: Vec<Box<dyn PadList>> = Vec::new();
        for &(count, base) in lists {
            let set = Log::default();
            logs.push(set.clone());
            boxed.push(Box::new(FakeList { count, base, set }));
        }
        (Chain::new(x, boxed), xset, logs)
    }

    /// The packet the chain answers for `user`, or the result when it is not success.
    fn packet(c: &mut Chain<FakeXInput>, user: u32) -> Result<u32, u32> {
        let mut state = [0xaa; 16];
        match c.get_state(user, &mut state) {
            ERROR_SUCCESS => Ok(PadState::from_bytes(&state).packet),
            other => {
                assert_eq!(state, [0xaa; 16], "user {user}: state kept");
                Err(other)
            }
        }
    }

    const GONE: Result<u32, u32> = Err(ERROR_DEVICE_NOT_CONNECTED);

    #[test]
    fn xinput_users_first_then_each_list() {
        // XInput users 1 and 3 plugged in, then a list of two and a list of one.
        let (mut c, _, _) = chain(Some(vec![1, 3]), &[(2, 200), (1, 300)]);
        let got: Vec<_> = (0..USERS).map(|u| packet(&mut c, u)).collect();
        assert_eq!(got, [Ok(101), Ok(103), Ok(200), Ok(201)]);
        let (mut c, _, _) = chain(Some(vec![]), &[(0, 200), (1, 300)]);
        let got: Vec<_> = (0..USERS).map(|u| packet(&mut c, u)).collect();
        assert_eq!(got, [Ok(300), GONE, GONE, GONE]);
    }

    /// With no XInput and no list pad, every user is "not connected"; with no XInput at all the
    /// lists alone.
    #[test]
    fn none_and_lists_alone() {
        let (mut c, _, _) = chain(None, &[]);
        assert_eq!(packet(&mut c, 0), GONE);
        let (mut c, _, _) = chain(Some(vec![]), &[(0, 200)]);
        assert_eq!(packet(&mut c, 0), GONE);
        let mut v = [0u8; 4];
        assert_eq!(c.set_state(0, &v), ERROR_DEVICE_NOT_CONNECTED);
        let (mut c, _, logs) = chain(None, &[(1, 200)]);
        assert_eq!(packet(&mut c, 0), Ok(200));
        assert_eq!(packet(&mut c, 1), GONE);
        v = [0x34, 0x12, 0x78, 0x56];
        assert_eq!(c.set_state(0, &v), ERROR_SUCCESS);
        assert_eq!(c.set_state(1, &v), ERROR_DEVICE_NOT_CONNECTED);
        assert_eq!(*logs[0].lock().unwrap(), [(0, 0x1234, 0x5678)]);
    }

    /// The vibration reaches the pad that is the user: XInput's with its own user number and
    /// bytes, a list's as the two speeds.
    #[test]
    fn rumble_goes_to_the_users_pad() {
        let (mut c, xset, logs) = chain(Some(vec![2]), &[(1, 200), (2, 300)]);
        let v = |n: u8| [n, 0, 0, n];
        assert_eq!(c.set_state(0, &v(1)), ERROR_SUCCESS);
        assert_eq!(c.set_state(1, &v(2)), ERROR_SUCCESS);
        assert_eq!(c.set_state(2, &v(3)), ERROR_SUCCESS);
        assert_eq!(c.set_state(3, &v(4)), ERROR_SUCCESS);
        assert_eq!(*xset.lock().unwrap(), [(2, v(1))]);
        assert_eq!(*logs[0].lock().unwrap(), [(0, 2, 0x200)]);
        assert_eq!(*logs[1].lock().unwrap(), [(0, 3, 0x300), (1, 4, 0x400)]);
    }

    /// A PlayStation pad rumbling as user 0 when an Xbox pad comes in before it (review of #783):
    /// the game's stop goes to the Xbox pad, and the PlayStation pad's motors are stopped too.
    #[test]
    fn a_pad_that_stops_being_the_user_stops_rumbling() {
        let xinput_on = Arc::new(Mutex::new(false));
        let xset: Log<(u32, [u8; 4])> = Log::default();
        /// XInput whose user 0 comes and goes.
        struct Toggle(Arc<Mutex<bool>>, Log<(u32, [u8; 4])>);
        impl PadSource for Toggle {
            fn get_state(&mut self, user: u32, state: &mut [u8; 16]) -> u32 {
                if user == 0 && *self.0.lock().unwrap() {
                    *state = [1; 16];
                    ERROR_SUCCESS
                } else {
                    ERROR_DEVICE_NOT_CONNECTED
                }
            }
            fn set_state(&mut self, user: u32, vibration: &[u8; 4]) -> u32 {
                self.1.lock().unwrap().push((user, *vibration));
                ERROR_SUCCESS
            }
        }
        let list_set = Log::default();
        let list = FakeList {
            count: 1,
            base: 200,
            set: list_set.clone(),
        };
        let mut c = Chain::new(
            Some(Toggle(xinput_on.clone(), xset.clone())),
            vec![Box::new(list) as Box<dyn PadList>],
        );
        assert_eq!(c.set_state(0, &[0, 0xff, 0, 0x80]), ERROR_SUCCESS);
        assert_eq!(*list_set.lock().unwrap(), [(0, 0xff00, 0x8000)]);
        *xinput_on.lock().unwrap() = true;
        assert_eq!(c.set_state(0, &[0; 4]), ERROR_SUCCESS);
        assert_eq!(*xset.lock().unwrap(), [(0, [0; 4])]);
        assert_eq!(*list_set.lock().unwrap(), [(0, 0xff00, 0x8000), (0, 0, 0)]);
        // Still the Xbox pad's: nothing more to the PlayStation pad.
        assert_eq!(c.set_state(0, &[0; 4]), ERROR_SUCCESS);
        assert_eq!(list_set.lock().unwrap().len(), 2);
    }

    #[test]
    fn find_places() {
        let (mut c, _, _) = chain(Some(vec![0]), &[(1, 200)]);
        let mut s = [0u8; 16];
        assert_eq!(c.find(0, &mut s), Place::XInput(0));
        assert_eq!(PadState::from_bytes(&s).packet, 100);
        let mut s = [7u8; 16];
        assert_eq!(c.find(1, &mut s), Place::List(0, 0));
        assert_eq!(s, [7u8; 16]);
        assert_eq!(c.find(2, &mut s), Place::None);
    }
}
