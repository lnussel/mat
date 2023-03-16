extern crate dbus;
extern crate notcurses;

use dbus::blocking::Connection;
use std::time::Duration;
use std::collections::HashMap;
use notcurses::{Notcurses,Received,Key,Style,Plane,Channel,Channels,Alpha,Position,Size};

mod machined;
use machined::manager::OrgFreedesktopMachine1Manager;

// https://en.opensuse.org/Help:Colors
// primary
const OPENSUSE_GREEN     :(u32, u32, u32, u32, u32) = (0x73ba25, 0x81c13b, 0x96cb5c, 0xb9dc92, 0xdceec8);
const OPENSUSE_DARK_BLUE :(u32, u32, u32, u32, u32) = (0x173f4f, 0x2f5361, 0x516f7b, 0x8b9fa7, 0xc5cfd3);
const OPENSUSE_CYAN      :(u32, u32, u32, u32, u32) = (0x35b9ab, 0x4ac0b4, 0x68cbc0, 0x9adcd5, 0xccedea);
// secondary
const OPENSUSE_DARK_CYAN :(u32, u32, u32, u32, u32) = (0x00a489, 0x1aad95, 0x40bba7, 0x7fd1c4, 0xbfe8e1);
const OPENSUSE_DARK_GREEN:(u32, u32, u32, u32, u32) = (0x6da741, 0x7cb054, 0x92bd71, 0xb6d3a0, 0xdae9cf);
const OPENSUSE_BLUE      :(u32, u32, u32, u32, u32) = (0x21a4df, 0x38ade2, 0x59bbe7, 0x90d1ef, 0xc7e8f7);
//const dialog_round: &str = "╭╮╰╯─│";
const BORDERS_ROUND: (&str, &str, &str, &str, &str, &str, &str, &str) = ("╭","╮","╰","╯","─","│","├","┤");
const BORDERS_LIGHT: (&str, &str, &str, &str, &str, &str, &str, &str) = ("┌","┐","└","┘","─","│","├","┤");

const SIZE_UNITS: [&str; 5] = ["", "k", "M", "G", "T"];


#[allow(dead_code)]
struct Machine {
    name: String,
    class: String,
    id: String,
    path: dbus::Path<'static>,
}

#[allow(dead_code)]
struct Image<'a> {
    name: String,
    t: String,
    ro: bool,
    t_created: u64,
    t_modified: u64,
    size: u64,
    path: dbus::Path<'static>,
    machine: Option<&'a Machine>,
}

fn update_listing(plane: &mut Plane, bus: &dbus::blocking::Proxy<'_, &dbus::blocking::Connection>) -> Result<(), Box<dyn std::error::Error>> {

    plane.into_ref_mut().erase();
    plane.cursor_home();

    let mut running = HashMap::new();

    if let Ok(l) = bus.list_machines() {
        for i in l {
            let m = Machine { name: i.0, class: i.1, id: i.2, path: i.3 };
            running.insert(m.name.clone(), m);
        }
    }
    let mut images: Vec<Image> = Vec::new();
    if let Ok(l) = bus.list_images() {
        for i in l {
            if i.0.starts_with('.') {
                continue;
            }
            let on = running.contains_key(&i.0);
            let img = Image { name: i.0.clone(), t: i.1, ro: i.2, t_created: i.3, t_modified: i.4, size: i.5, path: i.6, machine: if on {running.get(&i.0) } else {Option::None} };
            images.push(img);
        }
        images.sort_by(|a,b| a.name.cmp(&b.name));
    }

    for img in images {
            if img.machine.is_some() {
                let fg = plane.fg();
                plane.set_fg(0xFF0000);
                plane.putstr("❤️ ")?;
                plane.set_fg(fg);
                plane.on_styles(Style::Bold);
            } else {
                plane.putstr("  ")?;
            }
            let mut ss = "".to_string();
            if img.size > 1<<(10*(SIZE_UNITS.len())) {
                ss = "♾️".to_string();
            } else {
                for i in (0..SIZE_UNITS.len()).rev() {
                    if img.size > 1<<(10*i) {
                        ss = format!("{}{}", img.size>>(10*i), SIZE_UNITS[i]);
                        break;
                    }
                }
            }
            let mut name = img.name;
            // XXX: calculate available space
            if name.len() > 22 {
                name.truncate(20);
                name.push_str("..");
            }
            let s = format!("{:22} {} {:>5}", name, if img.ro { "ro" } else { "rw" }, ss);
            plane.putstrln(&s)?;
            if img.machine.is_some() {
                plane.off_styles(Style::Bold);
            }
    }
    Ok(())
}

struct Dialog {
    title: String,
    pos: Position,
    size: Size,
    has_shadow: bool,
    d: Plane,
    content: Plane,
}

impl Dialog {

    /*
    fn new(parent: &mut Plane) -> Result<Dialog, notcurses::Error> {
        let size = parent.size();
        let d = parent.new_child_sized_at(size, (0,0))?;
        Ok(Self { title: "".to_string(), pos: Position::new(1,1), size, has_shadow: true, d })
    }
    */

    fn new_sized_at(parent: &mut Plane, size: Size, pos: Position, shadow: bool) -> Result<Dialog, notcurses::Error> {
        let d = parent.new_child_sized_at(size, pos)?;
        // XXX: textara must be smaller with shadow
        let mut content = parent.new_child_sized_at((size.0-(if shadow {4} else {3}), size.1-3), (pos.0+1,pos.1+1))?;
        content.set_base(" ", Style::None, Channels::from_rgb(OPENSUSE_CYAN.0, OPENSUSE_DARK_BLUE.1))?;
        content.set_scrolling(true);
        Ok(Self { title: "".to_string(), pos, size, has_shadow: shadow, d, content})
    }


    fn draw_borders(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut d = &mut self.d;
        let size = d.size();
        let x = size.0;
        let y = size.1;
        let mut bxm = x-1;
        let mut bym = y-1;
        let b = BORDERS_LIGHT;

        if self.has_shadow {
            bxm = bxm - 2;
            bym = bym - 1;
        }

        //d.set_base("", Style::None, Channels::from_rgb_alpha(OPENSUSE_CYAN.4, Alpha::Transparent, OPENSUSE_DARK_BLUE.1, Alpha::Opaque))?;
        //d.move_above(&mut plane)?;
        //d.set_base_styles(Style::None)?;
        //d.set_base_channels(Channels::from_rgb(OPENSUSE_CYAN.4, OPENSUSE_DARK_BLUE.1))?;
        //d.set_base_fg(OPENSUSE_CYAN.4)?;
        //d.set_base_bg(OPENSUSE_CYAN.1)?;

        // upper left then line
        d.set_bg(OPENSUSE_DARK_BLUE.1);
        d.set_fg(OPENSUSE_DARK_CYAN.4);
        d.putstr(b.0)?;
        for n in 1..bxm {
            d.putstr(b.4)?;
        }
        // vertical left
        for i in 1..bym {
            d.putstr_at((0,i), b.5)?;
        }
        // lower left
        d.putstr_at((0,bym), b.2)?;
        let fg = d.fg();
        d.set_fg(0);
        // upper right
        d.putstr_at((bxm,0), b.1)?;
        // vertical right
        for i in 1..bym {
            d.putstr_at((bxm,i), b.5)?;
        }
        // lower horizontal line
        d.putstr_at((1,bym), b.4)?;
        for _ in 2..bxm {
            d.putstr(b.4)?;
        }
        d.putstr(b.3)?;
        d.set_fg(fg);

        if self.has_shadow {
            d.set_channels(Channels::from_rgb_alpha(0, Alpha::Transparent, 0, Alpha::Transparent));
            d.putstr_at((x-2,0), "  ")?;
            d.putstr_at((0, y-1), "  ")?;

            let bg = d.bg();
            d.set_channels(Channels::from_rgb_alpha(0, Alpha::Transparent, 1, Alpha::Opaque));

            // horizontal
            d.putstr_at((2,y-1), " ")?;
            for _ in 3..x {
                d.putstr(" ")?;
            }
            // vertical
            d.putstr_at((x-2,1), "  ")?;
            for i in 2..y {
                d.putstr_at((x-2,i), "  ")?;
            }

            d.set_bg(bg);
        }

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let mut nc = Notcurses::new()?;

    let mut plane = Plane::new(&mut nc)?;
    plane.set_base(" ", Style::None, Channels::from_rgb(OPENSUSE_CYAN.0, OPENSUSE_DARK_BLUE.0))?;

    let size = plane.size();
    let x = size.0 - 10;
    let y = size.1 - 5;

    //plane.putstr_at((x-1,1), "###")?;
    //plane.putstr_at((x-1,3), "###")?;

    //let mut d = plane.new_child_sized_at((x, y), (1,1))?;
    //let mut d = Dialog::new_sized_at(plane, (x, y), (1,1))?;

    let mut di = Dialog::new_sized_at(&mut plane, (x, y).into(), (1,1).into(), true)?;

    di.draw_borders()?;

    let conn = Connection::new_system()?;

    let bus = conn.with_proxy("org.freedesktop.machine1", "/org/freedesktop/machine1", Duration::from_millis(5000));

    update_listing(&mut di.content, &bus)?;

    plane.render()?;

    while let Ok(e) = nc.get_event() {
        match e.received {
            Received::Key(Key::Resize) => {},
//            Received::Key(notcurses::Received::Esc) => break,
            Received::Char('r') => {
                update_listing(&mut di.content, &bus)?;
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
