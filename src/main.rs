mod notifications;

use arboard::Clipboard;
use futures_util::stream::TryStreamExt;
use regex::Regex;
use std::time::Duration;
use zbus::{Connection, MessageStream};

#[tokio::main]
async fn main() -> zbus::Result<()> {
    let code_re = Regex::new(r"(?:\s|^)(?P<code>\d{6})(?:\s|\.|$)").unwrap();

    let connection = Connection::session().await?;

    // become a D-Bus monitor
    connection
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus.Monitoring"),
            "BecomeMonitor",
            &(&[] as &[&str], 0u32),
        )
        .await?;

    let connection2 = Connection::session().await?;
    let notification_proxy = notifications::NotificationsProxy::new(&connection2).await?;

    let mut stream = MessageStream::from(connection);
    while let Some(msg) = stream.try_next().await? {
        let header = msg.header();
        let message_type = header.message_type();

        if !matches!(message_type, zbus::MessageType::MethodCall) {
            continue;
        }

        let interface = header.interface();
        let path = header.path();
        let member = header.member();

        if let (Some(interface), Some(path), Some(member)) = (interface, path, member) {
            if interface != "org.freedesktop.Notifications"
                || path != "/org/freedesktop/Notifications"
                || member != "Notify"
            {
                continue;
            }

            let body = msg.body();
            let body: zbus::zvariant::Structure = body.deserialize()?;
            let fields = body.fields();

            let sender = &fields[0];
            let summary = &fields[3];
            let body = &fields[4];

            if !matches!(body, zbus::zvariant::Value::Str(_))
                || !matches!(sender, zbus::zvariant::Value::Str(_))
                || !matches!(summary, zbus::zvariant::Value::Str(_))
            {
                continue;
            }

            // if the sender is ourselves we skip the notification to avoid an infinite
            // loop
            let sender: &str = sender.try_into()?;
            if sender == "notifications-dbus-mon" {
                continue;
            }

            // if the notification is coming from the color picker just ignore
            let summary: &str = summary.try_into()?;
            if summary == "Color picker" {
                continue;
            }

            if let Some(caps) = code_re.captures(body.try_into()?) {
                let code = &caps["code"];

                let code_copied = code.to_owned();
                std::thread::spawn(move || {
                    let mut clipboard = Clipboard::new().unwrap();
                    clipboard.set_text(code_copied).unwrap();

                    // this thread has to stay alive or else the value will
                    // be dropped from the X clipboard. we'll allow 1 minute
                    // for the code to be consumed from the clipboard
                    std::thread::sleep(Duration::from_secs(60));
                });

                notification_proxy
                    .notify(
                        "notifications-dbus-mon",
                        0,
                        "",
                        "Code copied",
                        &format!("Code \"{code}\" copied to clipboard"),
                        &[],
                        std::collections::HashMap::new(),
                        5000,
                    )
                    .await?;

                println!("Copied code to clipboard");
            }
        }
    }

    Ok(())
}
