//! Real compositor verification over an owned solid-color background window.
use super::tests::wait;
use super::*;
use serde_json::Value;
use std::process::Command;

pub(super) fn hypr_json(command: &str) -> Value {
    let output = Command::new("hyprctl")
        .args(["-j", command])
        .output()
        .unwrap();
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).unwrap()
}
fn own_window(title: &str) -> Value {
    hypr_json("clients")
        .as_array()
        .unwrap()
        .iter()
        .find(|client| {
            client["pid"].as_u64() == Some(std::process::id() as u64)
                && client["title"].as_str() == Some(title)
        })
        .unwrap()
        .clone()
}
fn dispatch(command: &str, argument: &str) {
    let output = Command::new("hyprctl")
        .args(["dispatch", command, argument])
        .output()
        .unwrap();
    if output.status.success() {
        return;
    }
    // Hyprland 0.55 Lua dispatchers replace the legacy command syntax.
    let lua = match command {
        "togglefloating" => format!(
            "hl.dsp.window.float({{action=\"set\",window={}}})",
            serde_json::to_string(argument).unwrap()
        ),
        "focuswindow" => format!(
            "hl.dsp.focus({{window={}}})",
            serde_json::to_string(argument).unwrap()
        ),
        "resizewindowpixel" | "movewindowpixel" => {
            let (coordinates, address) = argument.split_once(",address:").unwrap();
            let numbers: Vec<i64> = coordinates
                .strip_prefix("exact ")
                .unwrap()
                .split_whitespace()
                .map(|v| v.parse().unwrap())
                .collect();
            let operation = if command == "resizewindowpixel" {
                "resize"
            } else {
                "move"
            };
            format!(
                "hl.dsp.window.{operation}({{x={},y={},relative=false,window=\"address:{address}\"}})",
                numbers[0], numbers[1]
            )
        }
        _ => panic!("Unsupported test dispatcher"),
    };
    let output = Command::new("hyprctl")
        .args(["dispatch", &lua])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Hyprland {command}: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
fn arrange(title: &str) {
    let window = own_window(title);
    let address = window["address"].as_str().unwrap();
    if window["floating"].as_bool() != Some(true) {
        dispatch("togglefloating", &format!("address:{address}"));
    }
    let monitors = hypr_json("monitors");
    let monitor = monitors
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["focused"].as_bool() == Some(true))
        .unwrap();
    let x = monitor["x"].as_i64().unwrap() + 80;
    let y = monitor["y"].as_i64().unwrap() + 80;
    dispatch(
        "resizewindowpixel",
        &format!("exact 800 520,address:{address}"),
    );
    dispatch(
        "movewindowpixel",
        &format!("exact {x} {y},address:{address}"),
    );
}
fn settle() {
    let start = Instant::now();
    wait(|| start.elapsed() >= Duration::from_millis(400));
}
fn capture(player: &PlayerWindow, path: &Path) -> f64 {
    let window = own_window(player.window.title().unwrap().as_str());
    let x = window["at"][0].as_i64().unwrap() + 20;
    let y = window["at"][1].as_i64().unwrap() + 20;
    let width = window["size"][0].as_i64().unwrap() - 40;
    let height = window["size"][1].as_i64().unwrap() - 40;
    assert!(width > 500 && height > 300);
    assert!(
        Command::new("grim")
            .args(["-g", &format!("{x},{y} {width}x{height}")])
            .arg(path)
            .status()
            .unwrap()
            .success()
    );
    let image = image::open(path).unwrap().to_rgb8();
    let point = (
        (image.width() as f64 * 0.8) as u32,
        (image.height() as f64 * 0.3) as u32,
    );
    let mut green = 0.0;
    for yy in point.1..point.1 + 10 {
        for xx in point.0..point.0 + 10 {
            green += image.get_pixel(xx, yy)[1] as f64;
        }
    }
    assert!(player.title.is_visible());
    let footer = player.hint.compute_bounds(&player.overlay).unwrap();
    if footer.y() + footer.height() > player.overlay.height() as f32 {
        let root = player.logo.parent().unwrap().parent().unwrap();
        eprintln!(
            "Root height {}, width {}, minimum {:?}",
            root.height(),
            root.width(),
            root.measure(gtk::Orientation::Vertical, root.width())
        );
        let mut child = root.first_child();
        while let Some(widget) = child {
            eprintln!(
                "{} visible {} bounds {:?} minimum {:?}",
                widget.type_().name(),
                widget.is_visible(),
                widget.compute_bounds(&player.overlay),
                widget.measure(gtk::Orientation::Vertical, widget.width())
            );
            child = widget.next_sibling();
        }
    }
    assert!(
        footer.y() + footer.height() <= player.overlay.height() as f32,
        "Footer clipped: y {}, height {}, overlay {}, window {}",
        footer.y(),
        footer.height(),
        player.overlay.height(),
        player.window.height()
    );
    green / 100.0
}
struct Windows {
    player: Rc<PlayerWindow>,
    background: gtk::ApplicationWindow,
}
impl Drop for Windows {
    fn drop(&mut self) {
        self.player.close();
        self.player.window.close();
        self.background.close();
    }
}

#[test]
#[ignore = "requires real Hyprland and grim; run this exact case separately"]
fn waveform_composes_with_desktop() {
    assert!(
        std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some(),
        "Real compositor verification requires Hyprland"
    );
    gtk::init().unwrap();
    let root = tempfile::tempdir().unwrap();
    let app = gtk::Application::builder()
        .application_id(format!(
            "io.github.aridev1.omamusi.Compositor{}",
            std::process::id()
        ))
        .build();
    app.register(None::<&gio::Cancellable>).unwrap();
    let background = gtk::ApplicationWindow::builder()
        .application(&app)
        .title("omaMusi compositor test background")
        .default_width(800)
        .default_height(520)
        .build();
    let canvas = gtk::DrawingArea::new();
    canvas.set_draw_func(|_, context, _, _| {
        context.set_source_rgb(0.05, 0.85, 0.20);
        context.paint().unwrap();
    });
    background.set_child(Some(&canvas));
    let mut cli = Cli::for_music(None);
    cli.view = VisualMode::Waveform;
    let player = PlayerWindow::new(
        &app,
        &cli,
        root.path().join("data"),
        root.path().join("home"),
    )
    .unwrap();
    let _windows = Windows {
        player: player.clone(),
        background: background.clone(),
    };
    background.present();
    player.window.present();
    wait(|| {
        hypr_json("clients")
            .as_array()
            .unwrap()
            .iter()
            .filter(|client| client["pid"].as_u64() == Some(std::process::id() as u64))
            .count()
            == 2
    });
    arrange("omaMusi compositor test background");
    arrange(player.window.title().unwrap().as_str());
    let address = own_window(player.window.title().unwrap().as_str())["address"]
        .as_str()
        .unwrap()
        .to_owned();
    dispatch("focuswindow", &format!("address:{address}"));
    settle();
    std::fs::create_dir_all("/tmp/omamusi-rust-compositor").unwrap();
    let wave = capture(
        &player,
        Path::new("/tmp/omamusi-rust-compositor/waveform.png"),
    );
    player.state.borrow_mut().visual.mode = VisualMode::Spectrum;
    settle();
    let opaque = capture(
        &player,
        Path::new("/tmp/omamusi-rust-compositor/spectrum.png"),
    );
    assert!(
        wave > 170.0 && wave - opaque > 80.0,
        "Desktop composition failed: waveform green {wave}, opaque green {opaque}"
    );
    assert_eq!(
        hypr_json("activewindow")["address"].as_str(),
        Some(address.as_str())
    );
    // Prove metadata and a long scrollable list fit at the supported minimum.
    let song = root.path().join("000.wav");
    super::tests::make_tone(&song);
    player.state.borrow_mut().audio.set_volume(0.0);
    let tracks = std::iter::once(song.clone())
        .chain((1..60).map(|i| {
            root.path().join(if i == 59 {
                format!("{}.flac", "VeryLongFilename".repeat(12))
            } else {
                format!("{i:03}.wav")
            })
        }))
        .collect();
    player.dispatch(Action::ReplaceQueue(tracks, Some(song)));
    wait(|| !player.state.borrow().model.loading);
    dispatch(
        "resizewindowpixel",
        &format!("exact 640 420,address:{address}"),
    );
    settle();
    let footer = player.hint.compute_bounds(&player.overlay).unwrap();
    assert!(
        footer.y() + footer.height() <= player.overlay.height() as f32,
        "Scrollable collection clipped footer at 640x420: y {}, height {}, stage {}",
        footer.y(),
        footer.height(),
        player.overlay.height()
    );
    assert!(
        player.panel.width() <= 340,
        "Long filenames expanded the sidebar to {}",
        player.panel.width()
    );
    for _ in 0..59 {
        player.handle_key(gdk::Key::j, gdk::ModifierType::empty());
    }
    settle();
    assert_eq!(player.state.borrow().model.selected, Some(59));
    assert!(player.scroller.vadjustment().value() > 0.0);
    player
        .last_interaction
        .set(Instant::now() - Duration::from_secs(6));
    player.refresh();
    settle();
    let transport = player.transport.compute_bounds(&player.overlay).unwrap();
    let gap = player.overlay.height() as f32 - transport.y() - transport.height();
    assert!(
        (20.0..=32.0).contains(&gap),
        "idle metadata not bottom aligned at 640x420: {gap}"
    );
    assert!(!player.footer.is_visible());
    capture(
        &player,
        Path::new("/tmp/omamusi-rust-compositor/idle-layout.png"),
    );
    player.interact();
    player.refresh();
    settle();
    assert!(player.footer.is_visible());
    eprintln!(
        "Real Hyprland composition passed: background visible through waveform ({wave:.0}), opaque spectrum ({opaque:.0}); player focused"
    );
}
