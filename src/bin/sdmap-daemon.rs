use std::{time, thread};
use std::sync::{Arc, Mutex};
use std::{fs::File, io::Write, io::Seek};
use evdev::{*, uinput::*, AbsoluteAxisType as Abs, RelativeAxisType as Rel};
use libc::input_absinfo;
use anyhow::Result;

use sdmap::VKBD_LAYOUT;

struct Daemon {
    dev_in: Device,
    absinfos_in: [input_absinfo; 64],
    cache_in: DeviceState,
    _devs_lizard: Vec<Device>,

    dev_out: VirtualDevice,
    kbd_mode: bool,

    scroll_daemon: ScrollDaemon,

    ipc: File,
}
impl Daemon {
    pub fn new() -> Result<Self> {
        let mut dev_in = evdev::enumerate()
            .find(|(_, d)| d.name() == Some("Steam Deck"))
            .unwrap().1;
        dev_in.grab()?;
        let absinfos_in = dev_in.get_abs_state()?;

        // grab lizard mode devices to make sure no events get through (e.g.
        // scroll events on left trackpad after waking steam deck from suspend)
        let _devs_lizard: Vec<Device> = evdev::enumerate()
            .filter(|(_, d)|  d.name() == Some("Valve Software Steam Controller"))
            .map(|(_, mut d)| { d.grab().unwrap(); d })
            .collect();

        let dev_out = VirtualDeviceBuilder::new()?
            .name("Steam Deck sdmapd main")
            .with_keys(&AttributeSet::from_iter(
                VKBD_LAYOUT.into_iter().flatten().flatten().chain([
                Key::KEY_LEFTMETA, Key::KEY_UP, Key::KEY_DOWN, Key::KEY_LEFT,
                Key::KEY_RIGHT, Key::KEY_LEFTSHIFT, Key::KEY_LEFTCTRL, Key::KEY_RIGHTALT,
                Key::KEY_LEFTALT, Key::KEY_TAB, Key::KEY_COMPOSE, Key::KEY_PAGEUP,
                Key::KEY_PAGEDOWN, Key::KEY_HOME, Key::KEY_END, Key::KEY_ENTER,
                Key::KEY_ESC, Key::KEY_BACKSPACE, Key::KEY_SPACE, Key::KEY_DELETE,
                Key::KEY_F1, Key::KEY_F2, Key::KEY_F3, Key::KEY_F4, Key::KEY_F5,
                Key::KEY_F6, Key::KEY_F7, Key::KEY_F8, Key::BTN_RIGHT, Key::BTN_LEFT,
                Key::BTN_MIDDLE,
            ])))?
            .with_relative_axes(&AttributeSet::from_iter([Rel::REL_X, Rel::REL_Y]))?
            .build()?;

        Ok(Self {
            absinfos_in,
            cache_in: dev_in.cached_state().clone(),
            dev_in,
            _devs_lizard,
            dev_out,
            kbd_mode: true,
            scroll_daemon: ScrollDaemon::new(absinfos_in[Abs::ABS_X.0 as usize].resolution)?,
            ipc: File::create("/run/sdmap")?,
        })
    }

    // Create a new Key event. (shortcut)
    fn new_key(key: Key, value: i32) -> InputEvent {
        InputEvent::new(EventType::KEY, key.0, value)
    }

    // Get current state of the input device. (shortcut)
    fn state_in(&self) -> &DeviceState {
        self.dev_in.cached_state()
    }

    // Create new Key events depending on the state of a key and a modifier.
    fn new_keymod(evt_in: InputEvent, modifier: bool, out_base: Key, out_mod: Key)
    -> Vec<InputEvent> {
        if evt_in.value() == 1 {
            vec!(Self::new_key(if modifier { out_mod } else { out_base }, 1))
        } else {
            vec!(Self::new_key(out_mod, 0), Self::new_key(out_base, 0))
        }
    }

    // Map an absolute event to a relative one.
    fn abs2rel(&self, evt_in: InputEvent, rel: Rel, coeff: f32) -> InputEvent {
        let absval = self.cache_in.abs_vals().unwrap()[evt_in.code() as usize];
        let delta = if evt_in.value() == 0 || absval.value == 0 { 0.0 }
                   else { (evt_in.value() - absval.value) as f32 * coeff } as i32;
        InputEvent::new(EventType::RELATIVE, rel.0, delta)
    }

    // Return the position on the trackpad keyboard based on the position of
    // ABS_HAT0. Return None if ABS_HAT0 isn't used.
    pub fn vkbd_xy(&self, old: bool) -> (usize, usize) {
        let absvals = if old {
            self.cache_in.abs_vals().unwrap()
        } else {
            self.state_in().abs_vals().unwrap()
        };
        let absinfo = self.absinfos_in[Abs::ABS_HAT0X.0 as usize];

        let absx = absvals[Abs::ABS_HAT0X.0 as usize].value;
        let absy = absvals[Abs::ABS_HAT0Y.0 as usize].value;
        if absx == 0 && absy == 0 {
            (usize::MAX, usize::MAX)
        } else {
            let vkbdy = (absy - absinfo.maximum).abs() * VKBD_LAYOUT.len() as i32
                        / ((absinfo.maximum * 2) + 1);
            let vkbdx = (absx + absinfo.maximum) * VKBD_LAYOUT[0].len() as i32
                        / ((absinfo.maximum * 2) + 1);
            (vkbdx as usize, vkbdy as usize)
        }
    }

    // Write the current trackpad keyboard position in the shared file.
    pub fn vkbd_send(&mut self) -> Result<()> {
        let old_keypos = self.vkbd_xy(true);
        let new_keypos = self.vkbd_xy(false);
        if old_keypos != new_keypos {
            self.ipc.rewind()?;
            self.ipc.set_len(0)?;
            self.ipc.write_all(format!("{} {}", new_keypos.0, new_keypos.1).as_bytes())?;
        }
        Ok(())
    }

    // Map a physical key to a key of the trackpad keyboard depending on the current
    // value of ABS_HAT0{X,Y}. If ABS_HAT0 isn't used send the `fallback_key`.
    fn key2vkbd(&self, evt_in: InputEvent, ki: usize, fallback_key: Key)
    -> Vec<InputEvent> {
        let keypos = self.vkbd_xy(false);
        if evt_in.value() == 0 {
            vec!()
        } else if self.vkbd_xy(false) != (usize::MAX, usize::MAX) {
            let key = VKBD_LAYOUT[keypos.1][keypos.0][ki];
            vec!(Self::new_key(key, 1), Self::new_key(key, 0))
        } else {
            vec!(Self::new_key(fallback_key, 1), Self::new_key(fallback_key, 0))
        }
    }

    fn remap(&mut self, evt_in: InputEvent) -> Vec<InputEvent> {
        let keyvals = self.state_in().key_vals().unwrap();
        let absvals = self.state_in().abs_vals().unwrap();
        let mod_th2 = keyvals.contains(Key::BTN_TRIGGER_HAPPY2);

        if evt_in.code() == Key::BTN_DPAD_UP.0 {
            Self::new_keymod(evt_in, mod_th2, Key::KEY_UP, Key::KEY_PAGEUP)
        } else if evt_in.code() == Key::BTN_DPAD_DOWN.0 {
            Self::new_keymod(evt_in, mod_th2, Key::KEY_DOWN, Key::KEY_PAGEDOWN)
        } else if evt_in.code() == Key::BTN_DPAD_LEFT.0 {
            Self::new_keymod(evt_in, mod_th2, Key::KEY_LEFT, Key::KEY_HOME)
        } else if evt_in.code() == Key::BTN_DPAD_RIGHT.0 {
            Self::new_keymod(evt_in, mod_th2, Key::KEY_RIGHT, Key::KEY_END)

        } else if evt_in.code() == Key::BTN_SELECT.0 {
            vec!(Self::new_key(Key::KEY_TAB, evt_in.value()))

        } else if evt_in.code() == Key::BTN_TL.0 {
            vec!(Self::new_key(Key::BTN_RIGHT, evt_in.value()))
        } else if evt_in.code() == Key::BTN_TR.0 {
            vec!(Self::new_key(Key::BTN_LEFT, evt_in.value()))
        } else if evt_in.code() == Key::BTN_TL2.0 {
            vec!(Self::new_key(Key::BTN_MIDDLE, evt_in.value()))
        } else if evt_in.code() == Key::BTN_TR2.0 {
            vec!(Self::new_key(Key::KEY_LEFTMETA, evt_in.value()))

        } else if evt_in.code() == Key::BTN_TRIGGER_HAPPY1.0 {
            vec!(Self::new_key(Key::KEY_LEFTSHIFT, evt_in.value()))
        } else if evt_in.code() == Key::BTN_TRIGGER_HAPPY3.0 {
            vec!(Self::new_key(Key::KEY_LEFTCTRL, evt_in.value()))
        } else if evt_in.code() == Key::BTN_TRIGGER_HAPPY2.0 {
            vec!(Self::new_key(Key::KEY_RIGHTALT, evt_in.value()))
        } else if evt_in.code() == Key::BTN_TRIGGER_HAPPY4.0 {
            vec!(Self::new_key(Key::KEY_LEFTALT, evt_in.value()))

        } else if evt_in.code() == Key::BTN_SOUTH.0 {
            self.key2vkbd(evt_in, 0, Key::KEY_ENTER)
        } else if evt_in.code() == Key::BTN_EAST.0 {
            self.key2vkbd(evt_in, 1, Key::KEY_ESC)
        } else if evt_in.code() == Key::BTN_NORTH.0 {
            self.key2vkbd(evt_in, 3, Key::KEY_BACKSPACE)
        } else if evt_in.code() == Key::BTN_WEST.0 {
            self.key2vkbd(evt_in, 2, Key::KEY_SPACE)
        } else if evt_in.code() == Key::BTN_START.0 {
            self.key2vkbd(evt_in, 4, Key::KEY_DELETE)
        } else if evt_in.code() == Key::BTN_BASE.0 {
            self.key2vkbd(evt_in, 5, Key::KEY_COMPOSE)
        } else if evt_in.code() == Key::BTN_THUMBR.0 {
            self.key2vkbd(evt_in, 6, Key::KEY_UNKNOWN)

        } else if evt_in.code() == Abs::ABS_HAT1X.0 {
            vec!(self.abs2rel(evt_in, Rel::REL_X, 0.01))
        } else if evt_in.code() == Abs::ABS_HAT1Y.0 {
            vec!(self.abs2rel(evt_in, Rel::REL_Y, -0.01))

        } else if evt_in.code() == Abs::ABS_X.0 || evt_in.code() == Abs::ABS_Y.0 {
            let absx = absvals[Abs::ABS_X.0 as usize].value;
            let absy = absvals[Abs::ABS_Y.0 as usize].value;
            self.scroll_daemon.scroll(absx, absy);
            vec!()

        } else {
            vec!()
        }
    }

    // Switch between desktop mode and gamepad mode if `BTN_THUMB` is pressed.
    fn switch_mode(&mut self, events_in: &[InputEvent]) {
        // Ensures `BTN_THUMB` has just been pushed (events_in) and that no other
        // buttons are currently held (state_in).
        if events_in.iter().any(|e| e.code() == Key::BTN_THUMB.0 && e.value() == 1)
        && self.state_in().key_vals().unwrap().iter().eq(vec![Key::BTN_THUMB]) {
            self.kbd_mode = !self.kbd_mode;
            if self.kbd_mode {
                self.dev_in.grab().unwrap();
            } else {
                self.scroll_daemon.scroll(0, 0);
                self.dev_in.ungrab().unwrap();
            }
        }
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            self.cache_in = self.dev_in.cached_state().clone();
            let events_in: Vec<InputEvent> = self.dev_in.fetch_events()?.collect();

            self.switch_mode(&events_in);
            if !self.kbd_mode { continue; }

            let events_out: Vec<InputEvent> = events_in.into_iter()
                .flat_map(|evt_in| self.remap(evt_in))
                .collect();
            self.dev_out.emit(&events_out)?;
            self.vkbd_send()?;
        }
    }
}

struct ScrollDaemon {
    scroll_thread: thread::JoinHandle<()>,
    scroll_xy: Arc<Mutex<(i32, i32)>>,
}
impl ScrollDaemon {
    pub fn new(resolution: i32) -> Result<Self> {
        let scroll_xy1 = Arc::new(Mutex::new((0, 0)));
        let scroll_xy2 = Arc::clone(&scroll_xy1);

        let mut dev_out = VirtualDeviceBuilder::new()?
            .name("Steam Deck sdmapd scroll")
            .with_relative_axes(&AttributeSet::from_iter(
                [Rel::REL_WHEEL, Rel::REL_HWHEEL]
            ))?
            .build()?;

        let scroll_thread = thread::spawn(move || {
            let scroll_xy = scroll_xy2;

            loop {
                let tmp_xy: (i32, i32) = *scroll_xy.lock().unwrap();
                if tmp_xy.0.abs() < resolution && tmp_xy.1.abs() < resolution {
                    thread::park();
                } else {
                    if tmp_xy.0.abs() >= resolution {
                        let val = if tmp_xy.0 > 0 { 1 } else { -1 };
                        let evt = InputEvent::new(EventType::RELATIVE, Rel::REL_HWHEEL.0, val);
                        dev_out.emit(&[evt]).unwrap();
                    }

                    if tmp_xy.1.abs() >= resolution {
                        let val = if tmp_xy.1 > 0 { -1 } else { 1 };
                        let evt = InputEvent::new(EventType::RELATIVE, Rel::REL_WHEEL.0, val);
                        dev_out.emit(&[evt]).unwrap();
                    }
                }

                thread::sleep(time::Duration::from_millis(100));
            }
        });

        Ok(Self { scroll_thread, scroll_xy: scroll_xy1 })
    }

    pub fn scroll(&self, x: i32, y: i32) {
        *self.scroll_xy.lock().unwrap() = (x, y);
        self.scroll_thread.thread().unpark();
    }
}

fn main() -> Result<()> {
    let mut daemon = Daemon::new()?;
    daemon.run()
}
