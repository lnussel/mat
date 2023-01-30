all: src/machined/manager.rs
	cargo b

src/machined/manager.rs: /usr/share/dbus-1/interfaces/org.freedesktop.machine1.Manager.xml
	dbus-codegen-0.10.0/target/debug/dbus-codegen-rust --file $^ > $@

.PHONY: all
