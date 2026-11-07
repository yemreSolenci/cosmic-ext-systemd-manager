mod app;
mod config;
mod systemd;
mod ui;

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<app::SystemdApplet>(())
}
