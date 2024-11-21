const BUTTONS: [char; 6] = ['a', 'b', 'y', 'x', 's', 'd'];

fn main() {
    let layout = sdmap::keysyms_layout();

    println!(r#"<!DOCTYPE html><html><body>
        <style type="text/css">
        body {{font-family:monospace; font-size:18px;}}
        table {{border-collapse: collapse; width:378px; height:378px;}}
        tr {{border:1px solid black;}}
        td {{text-align:center;}}
        td.a {{color:LimeGreen;}}
        td.b {{color:Crimson;}}
        td.x {{color:DodgerBlue;}}
        td.y {{color:Orange;}}
        td.s {{color:MediumPurple;}}
        td.d {{color:SlateGray; border-right:1px solid black;}}
        span.l0 {{font-size:30px; font-weight:bold;}}
        </style>
        <table>"#);
    for row in layout {
        println!("<tr>");
        for col in row {
            for (i, key) in col.iter().enumerate() {
                if i >= BUTTONS.len() { break; }
                let btn = BUTTONS[i];
                let l0 = key[0].clone();
                let print_mods = !"abcdefghijklmnopqrstuvwxyz".contains(&l0);
                let l1 = if print_mods { key[1].clone() }
                       else { "".into() };
                let l2 = if print_mods { key[2].clone() }
                       else { "".into() };
                println!("<td class=\"{btn}\">{l1} <span class=\"l0\">{l0}</span> {l2}</td>");
            }
        }
        println!("</tr>");
    }
    println!("</table></body></html>");
}
