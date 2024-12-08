use notify_rust::Notification;
use inotify::Inotify;
use std::{fs::File, io::Read, io::Seek};
use anyhow::Result;

const BUTTON_COLORS: [&str; 6] = ["LimeGreen", "Crimson", "DodgerBlue", "Orange",
    "MediumPurple", "SlateGray"];

fn pango_escape(character: &str) -> String {
    match character {
        "<" => "&lt;",
        ">" => "&gt;",
        k => k,
    }.into()
}

fn main() -> Result<()> {
    // partially flatten the layout and escape certain characters
    let mut layout_nm: [[String; 2]; 4] = Default::default();
    let mut layout_hl: [[String; 2]; 4] = Default::default();
    for (i, row) in sdmap::keysyms_layout().iter().enumerate() {
        for (j, col) in row.iter().enumerate() {
            for (k, key) in col[0..col.len() - 1].iter().enumerate() {
                let key = pango_escape(&key[0]);
                layout_nm[i][j].push_str(&key);
                layout_hl[i][j].push_str(&format!(
                    "<b><span foreground=\"{}\">{}</span></b>",
                    BUTTON_COLORS[k], &key
                ));
            }
        }
    }

    let mut ipc = File::open("/run/sdmap")?;

    let mut inotify = Inotify::init()?;
    let mut buffer = [0; 1024];
    inotify.watches().add("/run/sdmap", inotify::WatchMask::MODIFY)?;

    loop {
        let _ = inotify.read_events_blocking(&mut buffer)?;

        let mut vkbd_xy = String::new();
        ipc.rewind()?;
        ipc.read_to_string(&mut vkbd_xy)?;
        let vkbd_xy = vkbd_xy.strip_suffix("\n").unwrap_or(&vkbd_xy);
        if vkbd_xy.is_empty() { continue; }
        let vkbd_xy: Vec<&str> = vkbd_xy.split(' ').collect();
        let vkbd_xy: (usize, usize)  = (vkbd_xy[0].parse()?, vkbd_xy[1].parse()?);
        let mut result = String::new();
        for (i, row) in layout_nm.iter().enumerate() {
            for (j, col) in row.iter().enumerate() {
                let col = if vkbd_xy == (j, i) { &layout_hl[i][j] } else { col };
                result.push_str(&format!("{col}  "));
            }
            result.push('\n');
        }

        Notification::new()
            .summary("vkbd").body(&result).id(4000)
            .show()?;
    }
}
