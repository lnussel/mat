extern crate dbus;

use dbus::blocking::Connection;
use std::time::Duration;
use std::collections::HashMap;

mod machined;
use machined::manager::OrgFreedesktopMachine1Manager;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::new_system()?;

    let proxy = conn.with_proxy("org.freedesktop.machine1", "/org/freedesktop/machine1", Duration::from_millis(5000));

    let mut running = HashMap::new();

    if let Ok(l) = proxy.list_machines() {
        for i in l {
            let m = Machine { name: i.0, class: i.1, id: i.2, path: i.3 };
//            println!("Running: {} {} {} {}", m.name, m.class, m.id, m.path);
            running.insert(m.name.clone(), m);
        }
    }
    if let Ok(l) = proxy.list_images() {
        for i in l {
            let img = Image { name: i.0, t: i.1, ro: i.2, t_created: i.3, t_modified: i.4, size: i.5, path: i.6 };
            if img.name.starts_with('.') {
                continue;
            }
            let (ca, cb) = if running.contains_key(&img.name) { ("\x1b[32m", "\x1b[m") } else { ("", "")};
            println!("{}{} {} {} {}{}", ca, img.name, img.t, img.ro, img.size, cb);
        }
    }

    Ok(())
}
