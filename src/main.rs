use anyhow::Result;
use futures::StreamExt;
use minidsp::{
    Builder, Gain,
};
use inputbot::KeybdKey;
use tokio::sync::mpsc;

enum KbdAction {
    VolUp,
    VolDown,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut builder = Builder::new();
    builder.with_default_usb().unwrap();

    let devices: Vec<_> = builder
        // Probe each candidate device for its hardware id and serial number
        .probe()
        // Filter the list to keep the working devices
        .filter_map(|x| async move { x.ok() })
        .collect()
        .await;

    // Use the first device for further commands
    let dsp = devices
        .first()
        .expect("no devices found")
        .to_minidsp()
        .expect("unable to open device");

    let status = dsp.get_master_status().await?;
    println!("Master volume: {:.1}", status.volume.unwrap().0);

    let (tx, mut rx) = mpsc::channel(32);
    let tx2 = tx.clone();

    KeybdKey::F10Key.bind(move || {
        tx.blocking_send(KbdAction::VolDown).unwrap();
    });
    KeybdKey::F11Key.bind(move || {
        tx2.blocking_send(KbdAction::VolUp).unwrap();
    });

    std::thread::spawn(|| {
        inputbot::handle_input_events();
    });

    while let Some(msg) = rx.recv().await {
        let status = dsp.get_master_status().await?;
        let vol = status.volume.unwrap().0;
        match msg {
            KbdAction::VolUp => {
                dsp.set_master_volume(Gain(vol + 1.0)).await?;
            },
            KbdAction::VolDown => {
                dsp.set_master_volume(Gain(vol - 1.0)).await?;
            }
        }
    }

    Ok(())
}
