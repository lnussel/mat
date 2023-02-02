extern crate dbus;
extern crate notcurses;

use dbus::blocking::Connection;
use std::time::Duration;
use std::collections::HashMap;
use notcurses::{Notcurses,Received,Channel,Key,Style,Plane,Channels};

mod machined;
use machined::manager::OrgFreedesktopMachine1Manager;

// https://en.opensuse.org/Help:Colors
// primary
const opensuse_green     :(u32, u32, u32, u32, u32) = (0x73ba25, 0x81c13b, 0x96cb5c, 0xb9dc92, 0xdceec8);
const opensuse_dark_blue :(u32, u32, u32, u32, u32) = (0x173f4f, 0x2f5361, 0x516f7b, 0x8b9fa7, 0xc5cfd3);
const opensuse_cyan      :(u32, u32, u32, u32, u32) = (0x35b9ab, 0x4ac0b4, 0x68cbc0, 0x9adcd5, 0xccedea);
// secondary
const opensuse_dark_cyan :(u32, u32, u32, u32, u32) = (0x00a489, 0x1aad95, 0x40bba7, 0x7fd1c4, 0xbfe8e1);
const opensuse_dark_green:(u32, u32, u32, u32, u32) = (0x6da741, 0x7cb054, 0x92bd71, 0xb6d3a0, 0xdae9cf);
const opensuse_blue      :(u32, u32, u32, u32, u32) = (0x21a4df, 0x38ade2, 0x59bbe7, 0x90d1ef, 0xc7e8f7);
//const dialog_round: &str = "╭╮╰╯─│";
const borders_round: (&str, &str, &str, &str, &str, &str, &str, &str) = ("╭","╮","╰","╯","─","│","├","┤");

struct Machine {
    name: String,
    class: String,
    id: String,
    path: dbus::Path<'static>,
}

struct Image {
    name: String,
    t: String,
    ro: bool,
    t_created: u64,
    t_modified: u64,
    size: u64,
    path: dbus::Path<'static>,
}

fn update_listing(plane: &mut Plane, bus: &dbus::blocking::Proxy<'_, &dbus::blocking::Connection>) -> Result<(), Box<dyn std::error::Error>> {
    let mut running = HashMap::new();

    if let Ok(l) = bus.list_machines() {
        for i in l {
            let m = Machine { name: i.0, class: i.1, id: i.2, path: i.3 };
//            println!("Running: {} {} {} {}", m.name, m.class, m.id, m.path);
            running.insert(m.name.clone(), m);
        }
    }
    if let Ok(l) = bus.list_images() {
        for i in l {
            let img = Image { name: i.0, t: i.1, ro: i.2, t_created: i.3, t_modified: i.4, size: i.5, path: i.6 };
            if img.name.starts_with('.') {
                continue;
            }
            let (ca, cb) = if running.contains_key(&img.name) { ("\x1b[32m", "\x1b[m") } else { ("", "")};
            let s = format!("{}{} {} {} {}{}", ca, img.name, img.t, img.ro, img.size, cb);
            plane.putstrln(&s)?;
            //println!("{}{} {} {} {}{}", ca, img.name, img.t, img.ro, img.size, cb);
        }
    }
    Ok(())
}

fn draw_borders(d: &mut Plane) -> Result<(), Box<dyn std::error::Error>> {
    let size = d.size();
    let x = size.0;
    let y = size.1;

    d.putstr(borders_round.0)?;
    for i in (1..x-1) {
        d.putstr(borders_round.4)?;
    }
    for i in (1..y-1) {
        d.putstr_at((0,i), borders_round.5)?;
    }
    d.putstr_at((0,y-1), borders_round.2)?;
    d.set_fg(0);
    d.putstr_at((x-1,0), borders_round.1)?;
    for i in (1..y-1) {
        d.putstr_at((x-1,i), borders_round.5)?;
    }
    d.putstr_at((1,y-1), borders_round.4)?;
    for i in (1..x-2) {
        d.putstr(borders_round.4)?;
    }
    d.putstr(borders_round.3)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let mut nc = Notcurses::new()?;

    let mut plane = Plane::new(&mut nc)?;
    plane.set_base(" ", Style::None, Channels::from_rgb(opensuse_cyan.0, opensuse_dark_blue.0))?;
    let bc = plane.base()?;
//    plane.into_ref_mut().erase();

    let size = plane.size();
    let x = size.0;
    let y = size.1;
    let mut d = plane.new_child_sized_at((x-2, y-2), (1,1))?;
    d.set_base(" ", Style::None, Channels::from_rgb(opensuse_cyan.4, opensuse_dark_blue.1))?;
    draw_borders(&mut d)?;

    let mut textarea = plane.new_child_sized_at((x-4, y-4), (2,2))?;
    textarea.set_base(" ", Style::None, Channels::from_rgb(opensuse_cyan.0, opensuse_dark_blue.1))?;
    textarea.set_scrolling(true);

    let conn = Connection::new_system()?;

    let bus = conn.with_proxy("org.freedesktop.machine1", "/org/freedesktop/machine1", Duration::from_millis(5000));

    update_listing(&mut textarea, &bus)?;

    plane.render()?;

    while let Ok(e) = nc.get_event() {
        match e.received {
            Received::Key(Key::Resize) => {},
//            Received::Key(notcurses::Received::Esc) => break,
            Received::Char('r') => {
                update_listing(&mut plane, &bus)?;
            },
            Received::Char('q') => break,
            _ => {
                return Err(format!("Invalid event {}", e).into());
            },
        }
        plane.render()?;
    }

    Ok(())
}
