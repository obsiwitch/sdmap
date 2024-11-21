use evdev::Key;
use xkbcommon::xkb;

// Trackpad keyboard layout. Unused key slots should be mapped to KEY_UNKNOWN.
pub const VKBD_LAYOUT: [[[Key; 7]; 2]; 4] = [
    [[Key::KEY_1, Key::KEY_2, Key::KEY_3, Key::KEY_4, Key::KEY_5, Key::KEY_MINUS, Key::KEY_F1],
     [Key::KEY_6, Key::KEY_7, Key::KEY_8, Key::KEY_9, Key::KEY_0, Key::KEY_EQUAL, Key::KEY_F2]],
    [[Key::KEY_Q, Key::KEY_W, Key::KEY_E, Key::KEY_R, Key::KEY_T, Key::KEY_LEFTBRACE, Key::KEY_F3],
     [Key::KEY_Y, Key::KEY_U, Key::KEY_I, Key::KEY_O, Key::KEY_P, Key::KEY_RIGHTBRACE, Key::KEY_F4]],
    [[Key::KEY_A, Key::KEY_S, Key::KEY_D, Key::KEY_F, Key::KEY_G, Key::KEY_APOSTROPHE, Key::KEY_F5],
     [Key::KEY_H, Key::KEY_J, Key::KEY_K, Key::KEY_L, Key::KEY_SEMICOLON, Key::KEY_BACKSLASH, Key::KEY_F6]],
    [[Key::KEY_Z, Key::KEY_X, Key::KEY_C, Key::KEY_V, Key::KEY_B, Key::KEY_GRAVE, Key::KEY_F7],
     [Key::KEY_N, Key::KEY_M, Key::KEY_COMMA, Key::KEY_DOT, Key::KEY_SLASH, Key::KEY_102ND, Key::KEY_F8]]
];

// ---

const EVDEV_OFFSET: u32 = 8;

// Change the xkb state by pushing/releasing key modifiers.
fn xkb_mods(state: &mut xkb::State, mods: &[Key], enable: bool) {
    for keymod in mods {
        state.update_key(
            keymod.0 as u32 + EVDEV_OFFSET,
            if enable { xkb::KeyDirection::Down }
            else { xkb::KeyDirection::Up },
        );
    }
}

// Get a string representation (character/symbol) of a keyboard key.
fn xkb_keysym(state: &mut xkb::State, evkey: Key, mods: &[Key]) -> String {
    if evkey == Key::KEY_UNKNOWN {
        return "".into();
    }

    xkb_mods(state, mods, true);
    let mut result = state.key_get_utf8(evkey.0 as u32 + EVDEV_OFFSET);
    if result.is_empty() {
        let keysym = state.key_get_one_sym(evkey.0 as u32 + EVDEV_OFFSET);
        result = xkb::keysym_get_name(keysym);
    }
    xkb_mods(state, mods, false);

    match result.as_str() {
        "dead_circumflex" => "^",
        "dead_acute"      => "´",
        "dead_grave"      => "`",
        "dead_diaeresis"  => "¨",
        "dead_belowdot"   => ".",
        result => result,
    }.into()
}

// Get the trackpad keyboard layout (VKBD_LAYOUT) as a nested array of strings.
pub fn keysyms_layout() -> [[[[String; 3]; 7]; 2]; 4] {
    let ctx = xkb::Context::new(0);
    let keymap = xkb::Keymap::new_from_names(&ctx, "", "", "fr", "", None, 0).unwrap();
    let mut state = xkb::State::new(&keymap);

    let mut result: [[[[String; 3]; 7]; 2]; 4] = Default::default();
    for (i, row) in VKBD_LAYOUT.into_iter().enumerate() {
        for (j, col) in row.into_iter().enumerate() {
            for (k, key) in col.into_iter().enumerate() {
                result[i][j][k][0] = xkb_keysym(&mut state, key, &[]);
                result[i][j][k][1] = xkb_keysym(&mut state, key, &[Key::KEY_LEFTSHIFT]);
                result[i][j][k][2] = xkb_keysym(&mut state, key, &[Key::KEY_RIGHTALT]);
            }
        }
    }
    result
}
