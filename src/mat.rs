extern crate dbus;
extern crate notcurses;

use std::process::Command;
use dbus::blocking::Connection;
use std::time::Duration;
use std::collections::HashMap;
use notcurses::{Notcurses,Received,Key,Style,Plane,Channel,Channels,Alpha,Position,Size};

mod machined;
use machined::manager::OrgFreedesktopMachine1Manager;
mod systemd;
use systemd::manager::OrgFreedesktopSystemd1Manager;

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
struct Image {
    name: String,
    t: String,
    ro: bool,
    t_created: u64,
    t_modified: u64,
    size: u64,
    path: dbus::Path<'static>,
    machine: Option<Machine>,
}

fn update_images(images: &mut Vec<Image>, machined: &dbus::blocking::Proxy<'_, &dbus::blocking::Connection>) -> Result<(), Box<dyn std::error::Error>> {

    let mut running = HashMap::new();

    if let Ok(l) = machined.list_machines() {
        for i in l {
            let m = Machine { name: i.0, class: i.1, id: i.2, path: i.3 };
            running.insert(m.name.clone(), m);
        }
    }
    if let Ok(l) = machined.list_images() {
        images.clear();
        for i in l {
            if i.0.starts_with('.') {
                continue;
            }
            let on = running.contains_key(&i.0);
            let img = Image { name: i.0.clone(), t: i.1, ro: i.2, t_created: i.3, t_modified: i.4, size: i.5, path: i.6, machine: if on {running.remove(&i.0) } else {Option::None} };
            images.push(img);
        }
        images.sort_by(|a,b| a.name.cmp(&b.name));
    }
    Ok(())
}

fn draw_images(plane: &mut Plane, images: &Vec<Image>, current: usize) -> Result<(), Box<dyn std::error::Error>> {
    plane.into_ref_mut().erase();
    plane.cursor_home();

    let mut idx: usize = 0;
    for img in images {
            let bg = plane.bg();
            if idx == current {
                plane.set_bg(OPENSUSE_DARK_BLUE.2);
            }
            plane.cursor_move_to((0, idx));
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
                ss = "-".to_string();
            } else {
                for i in (0..SIZE_UNITS.len()).rev() {
                    if img.size > 1<<(10*i) {
                        ss = format!("{}{}", img.size>>(10*i), SIZE_UNITS[i]);
                        break;
                    }
                }
            }
            let mut name = img.name.clone();
            // XXX: calculate available space
            let maxlen: usize = plane.size().0 as usize - 11;
            if name.len() > maxlen {
                name.truncate(maxlen - 2);
                name.push_str("..");
            }
            let s = format!("{:maxlen$} {} {:>5}", name, if img.ro { "ro" } else { "rw" }, ss);
            plane.putstr(&s)?;
            if img.machine.is_some() {
                plane.off_styles(Style::Bold);
            }
            if idx == current {
                plane.set_bg(bg);
            }
            idx += 1;
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

    fn new_sized_at(parent: &mut Plane, size: Size, pos: Position, shadow: bool) -> Result<Dialog, Box<dyn std::error::Error>> {
        let d = parent.new_child_sized_at(size, pos)?;
        let mut content = parent.new_child_sized_at((size.0-(if shadow {4} else {3}), size.1-3), (pos.0+1,pos.1+1))?;
        content.set_base(" ", Style::None, Channels::from_rgb(OPENSUSE_CYAN.0, OPENSUSE_DARK_BLUE.1))?;
        content.set_scrolling(true);

        let mut di = Self { title: "".to_string(), pos, size, has_shadow: shadow, d, content};
        di.draw_borders()?;

        Ok(di)
    }

    fn new_centered_text(parent: &mut Plane, text: &str, shadow: bool) -> Result<Dialog, Box<dyn std::error::Error>> {
        let needed = text.len() as u32;
        let size = parent.size();
        let x = size.0/2-needed/2;
        let y = size.1/2;
        let h = 1;
        let w = needed;
        let d = parent.new_child_sized_at((w + if shadow {4} else {2}, h + if shadow {3} else {2}), (x-1, y-1))?;
        let mut content = parent.new_child_sized_at((w, h),(x, y))?;
        content.set_base(" ", Style::None, Channels::from_rgb(OPENSUSE_CYAN.0, OPENSUSE_DARK_BLUE.1))?;
        content.putstr(&text)?;


        let mut di = Self { title: "".to_string(), pos: d.position(), size: d.size(), has_shadow: shadow, d, content};
        di.draw_borders()?;

        Ok(di)
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

    let mut cmd = Command::new("machinectl");

    {
        let mut nc = Notcurses::new()?;

        let mut plane = Plane::new(&mut nc)?;
        plane.set_base(" ", Style::None, Channels::from_rgb(OPENSUSE_CYAN.0, OPENSUSE_DARK_BLUE.0))?;

        let size = plane.size();

        let conn = Connection::new_system()?;

        let machined = conn.with_proxy("org.freedesktop.machine1", "/org/freedesktop/machine1", Duration::from_millis(5000));

        let mut images: Vec<Image> = Vec::new();
        update_images(&mut images, &machined)?;

        if images.len() == 0 {
            let mut txt = Dialog::new_centered_text(&mut plane, "No images found", true)?;
            plane.render()?;
            nc.get_event();
            return Err("No images found, read https://en.opensuse.org/Systemd-machined".into());
        }

        plane.putstr_at((1,size.1-1), "Enter: Start/Stop, Right: Shell, F5: Refresh, q: quit")?;

        let mut di = Dialog::new_sized_at(&mut plane, (size.0-2, size.1-3).into(), (1,1).into(), true)?;
        let mut current: usize = 0;
        draw_images(&mut di.content, &images, current);

        let systemd = conn.with_proxy("org.freedesktop.systemd1", "/org/freedesktop/systemd1", Duration::from_millis(5000));

        plane.render()?;

        while let Ok(e) = nc.get_event() {
            let mut update = false;
            let mut redraw = false;
            match e.received {
                Received::Key(Key::Right) => {
                    if images.len() > 0 && images[current].machine.is_some() {
                        cmd.arg("shell").arg(images[current].name.clone());
                        break;
                    }
                }
                Received::Char('r') => {
                    if images.len() > 0 && images[current].machine.is_some() {
                        // reboot
                        machined.kill_machine(&images[current].name, "leader", 2 /* SIGINT */);
                    }
                }
                Received::Key(Key::Enter) => {
                    if images.len() > 0 {
                        if images[current].machine.is_some() {
                            // poweroff
                            machined.kill_machine(&images[current].name, "leader", 38 /* SIGRTMIN+4 */);
                            let mut txt = Dialog::new_centered_text(&mut plane, "powering off", true)?;
                            plane.render()?;
                            // FIXME
                            notcurses::sleep![2,0];
                        } else {
                            let name = format!("systemd-nspawn@{}.service", images[current].name);
                            systemd.start_unit(&name, "fail");
                            let mut txt = Dialog::new_centered_text(&mut plane, "starting", true)?;
                            plane.render()?;
                            // FIXME
                            notcurses::sleep![2,0];
                        }
                        update = true;
                    }
                },
                Received::Key(Key::Resize) => {},
                //            Received::Key(notcurses::Received::Esc) => break,
                Received::Key(Key::F05) => {
                    current = 0;
                    redraw = true;
                },
                Received::Char('q') => break,
                Received::Key(Key::Up) => {
                    if current > 0 {
                        current -= 1;
                    }
                    redraw = true;
                },
                Received::Key(Key::Down) => {
                    if current + 1 < images.len() {
                        current += 1;
                    }
                    redraw = true;
                },
                _ => {
                    return Err(format!("Invalid event {}", e).into());
                },
            }
            if update {
                update_images(&mut images, &machined)?;
                redraw = true;
            }
            if redraw {
                    draw_images(&mut di.content, &images, current);
            }
            plane.render()?;
        }
    }

    if cmd.get_args().len() > 0 {
        cmd.status();
    }

    Ok(())
}
