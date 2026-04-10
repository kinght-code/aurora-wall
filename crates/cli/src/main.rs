use aurora_wall_backend_api::{LoopMode, ScalingMode, TransitionMode, WallpaperKind, WallpaperSpec};
use aurora_wall_config::{default_config_path, AppConfig};
use aurora_wall_daemon::{
    apply_config, arch_install_hint, default_grub_theme_dir, default_plymouth_theme_dir,
    export_grub_theme, export_plymouth_theme, export_video_poster, install_boot_theme,
    list_outputs, pkgbuild_template, sync_wal_theme, systemd_user_service, upsert_wallpaper,
    write_default_config, DaemonStatus, RuntimeStatus,
};
use aurora_wall_ipc::SocketPath;
use aurora_wall_state::{default_state_path, AppliedState};
use std::env;
use std::path::PathBuf;
use std::process;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {}", error);
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".to_string());
    let rest: Vec<String> = args.collect();

    match command.as_str() {
        "help" | "--help" | "-h" => print_help(),
        "doctor" => doctor()?,
        "init-config" => init_config(rest)?,
        "show-config" => show_config(rest)?,
        "list-outputs" => list_outputs_command()?,
        "remove-output" => remove_output(rest)?,
        "set-image" => set_image(rest)?,
        "set-video" => set_video(rest)?,
        "apply" => apply(rest)?,
        "export-video-poster" => export_video_poster_command(rest)?,
        "export-boot-theme" => export_boot_theme_command(rest)?,
        "install-boot-theme" => install_boot_theme_command(rest)?,
        "export-grub-theme" => export_grub_theme_command(rest)?,
        "export-plymouth-theme" => export_plymouth_theme_command(rest)?,
        "status" => status(rest)?,
        "print-service" => print!("{}", systemd_user_service()),
        "print-pkgbuild" => print!("{}", pkgbuild_template()),
        other => {
            return Err(format!(
                "unknown command: {}. Run `aurora-wall help` for usage.",
                other
            ))
        }
    }

    Ok(())
}

fn print_help() {
    println!("aurora-wall");
    println!("  doctor");
    println!("  init-config [--config PATH]");
    println!("  show-config [--config PATH]");
    println!("  list-outputs");
    println!("  remove-output --output NAME [--config PATH]");
    println!("  set-image --output NAME --path FILE [--scaling fill|fit|center] [--transition none|fade] [--config PATH]");
    println!("  set-video --output NAME --path FILE [--loop infinite|once] [--mute yes|no] [--config PATH]");
    println!("  apply [--config PATH] [--restore] [--no-wal] [-v|--verbose]");
    println!("  export-video-poster --video FILE [--output FILE] [--at 00:00:03]");
    println!("  export-boot-theme --video FILE [--poster FILE] [--grub-dir PATH] [--plymouth-dir PATH] [--title TEXT] [--at 00:00:03]");
    println!("  install-boot-theme [--grub-dir PATH] [--plymouth-dir PATH]");
    println!("  export-grub-theme [--config PATH] [--theme-dir PATH] [--background FILE] [--title TEXT]");
    println!("  export-plymouth-theme [--config PATH] [--theme-dir PATH] [--background FILE] [--video FILE] [--title TEXT] [--at 00:00:03]");
    println!("  status [--config PATH]");
    println!("  print-service");
    println!("  print-pkgbuild");
}

fn doctor() -> Result<(), String> {
    let runtime = RuntimeStatus::load(None).map_err(|error| error.to_string())?;
    println!("backend: hyprland");
    println!("desktop_session: {}", runtime.environment.desktop_session.as_deref().unwrap_or("unset"));
    println!("current_desktop: {}", runtime.environment.current_desktop.as_deref().unwrap_or("unset"));
    println!("wayland_display: {}", runtime.environment.wayland_display.as_deref().unwrap_or("unset"));
    println!(
        "hyprland_instance_signature: {}",
        runtime
            .environment
            .hyprland_instance_signature
            .as_deref()
            .unwrap_or("unset")
    );
    println!("is_hyprland: {}", yes_no(runtime.environment.is_hyprland()));
    println!(
        "live_session_ready: {}",
        yes_no(runtime.environment.is_live_session_ready())
    );
    println!("config_path: {}", runtime.config_path.display());
    println!("state_path: {}", runtime.state_path.display());
    for line in runtime.dependencies.summary_lines() {
        println!("{}", line);
    }
    println!("configured_wallpapers: {}", runtime.configured_wallpapers);
    println!("install_hint: {}", arch_install_hint());
    Ok(())
}

fn init_config(args: Vec<String>) -> Result<(), String> {
    let config_path = config_path_from_args(&args);
    let config = write_default_config(&config_path).map_err(|error| error.to_string())?;
    println!("wrote config: {}", config_path.display());
    println!("library_dir: {}", config.library_dir.display());
    Ok(())
}

fn show_config(args: Vec<String>) -> Result<(), String> {
    let config_path = config_path_from_args(&args);
    let config = AppConfig::load_or_default(&config_path).map_err(|error| error.to_string())?;
    println!("config_path: {}", config_path.display());
    println!("backend: {}", config.preferred_backend);
    println!("restore_on_login: {}", config.restore_on_login);
    println!("library_dir: {}", config.library_dir.display());
    for wallpaper in config.wallpapers {
        println!(
            "wallpaper: output={} kind={} path={} scaling={} transition={} muted={} loop={}",
            wallpaper.output,
            wallpaper.kind.as_str(),
            wallpaper.path,
            wallpaper.scaling.as_str(),
            wallpaper.transition.as_str(),
            wallpaper.muted,
            wallpaper.loop_mode.as_str()
        );
    }
    Ok(())
}

fn list_outputs_command() -> Result<(), String> {
    match list_outputs() {
        Ok(outputs) => {
            if outputs.is_empty() {
                println!("no outputs reported by hyprctl");
            } else {
                for output in outputs {
                    println!("{}", output);
                }
            }
            Ok(())
        }
        Err(error) => Err(format!(
            "unable to list Hyprland outputs: {}. Run this inside an active Hyprland session.",
            error
        )),
    }
}

fn set_image(args: Vec<String>) -> Result<(), String> {
    let output = required_flag(&args, "--output")?;
    let path = required_flag(&args, "--path")?;
    validate_output_if_possible(&output)?;
    let scaling = optional_flag(&args, "--scaling")
        .as_deref()
        .and_then(ScalingMode::parse)
        .unwrap_or(ScalingMode::Fill);
    let transition = optional_flag(&args, "--transition")
        .as_deref()
        .and_then(TransitionMode::parse)
        .unwrap_or(TransitionMode::Fade);
    let config_path = config_path_from_args(&args);

    let mut config = AppConfig::load_or_default(&config_path).map_err(|error| error.to_string())?;
    let spec = WallpaperSpec {
        output,
        kind: WallpaperKind::Image,
        path,
        scaling,
        transition,
        muted: true,
        loop_mode: LoopMode::Infinite,
    };
    spec.validate()?;
    upsert_wallpaper(&mut config, spec);
    config.save(&config_path).map_err(|error| error.to_string())?;
    println!("saved image wallpaper to {}", config_path.display());
    Ok(())
}

fn set_video(args: Vec<String>) -> Result<(), String> {
    let output = required_flag(&args, "--output")?;
    let path = required_flag(&args, "--path")?;
    validate_output_if_possible(&output)?;
    let loop_mode = optional_flag(&args, "--loop")
        .as_deref()
        .and_then(LoopMode::parse)
        .unwrap_or(LoopMode::Infinite);
    let muted = optional_flag(&args, "--mute")
        .map(|value| parse_yes_no(&value))
        .transpose()?
        .unwrap_or(true);
    let config_path = config_path_from_args(&args);

    let mut config = AppConfig::load_or_default(&config_path).map_err(|error| error.to_string())?;
    let spec = WallpaperSpec {
        output,
        kind: WallpaperKind::Video,
        path,
        scaling: ScalingMode::Fill,
        transition: TransitionMode::None,
        muted,
        loop_mode,
    };
    spec.validate()?;
    upsert_wallpaper(&mut config, spec);
    config.save(&config_path).map_err(|error| error.to_string())?;
    println!("saved video wallpaper to {}", config_path.display());
    Ok(())
}

fn remove_output(args: Vec<String>) -> Result<(), String> {
    let output = required_flag(&args, "--output")?;
    let config_path = config_path_from_args(&args);
    let mut config = AppConfig::load_or_default(&config_path).map_err(|error| error.to_string())?;
    let before = config.wallpapers.len();
    config.wallpapers.retain(|wallpaper| wallpaper.output != output);
    config.save(&config_path).map_err(|error| error.to_string())?;
    println!(
        "removed {} entries for output {} from {}",
        before.saturating_sub(config.wallpapers.len()),
        output,
        config_path.display()
    );
    Ok(())
}

fn apply(args: Vec<String>) -> Result<(), String> {
    let config_path = config_path_from_args(&args);
    let config = AppConfig::load_or_default(&config_path).map_err(|error| error.to_string())?;
    let verbose = has_flag(&args, "-v") || has_flag(&args, "--verbose");
    let sync_wal = !has_flag(&args, "--no-wal");

    if args.iter().any(|arg| arg == "--restore") {
        let state_path = default_state_path();
        if let Ok(state) = AppliedState::load(&state_path) {
            if verbose {
                println!(
                    "restoring previous backend={} applied_items={}",
                    state.last_applied_backend, state.applied_items
                );
            }
        }
    }

    let actions = apply_config(&config, verbose).map_err(|error| error.to_string())?;
    let wal_action = if sync_wal {
        sync_wal_theme(&config, verbose).map_err(|error| error.to_string())?
    } else {
        None
    };
    if actions.is_empty() {
        println!("applied");
    } else if verbose {
        for action in actions {
            println!("applied: {}", action);
        }
        if let Some(action) = wal_action {
            println!("applied: {}", action);
        }
    } else {
        println!("applied");
    }
    Ok(())
}

fn status(args: Vec<String>) -> Result<(), String> {
    let config_path = config_path_from_args(&args);
    let config = AppConfig::load_or_default(&config_path).map_err(|error| error.to_string())?;
    let runtime = RuntimeStatus::load(Some(&config_path)).map_err(|error| error.to_string())?;
    let state_path = default_state_path();
    let state = AppliedState::load(&state_path).ok();
    let socket = SocketPath::default();
    let daemon = DaemonStatus::planned();

    println!("status: {}", daemon.summary());
    println!("boot_summary: {}", DaemonStatus::boot_summary(&config));
    println!("ipc_socket: {}", socket.as_path().display());
    println!("config_path: {}", config_path.display());
    println!("state_path: {}", state_path.display());
    println!("configured_wallpapers: {}", config.wallpapers.len());
    println!("live_session_ready: {}", yes_no(runtime.environment.is_live_session_ready()));
    if let Some(state) = state {
        println!("last_applied_backend: {}", state.last_applied_backend);
        println!("last_applied_items: {}", state.applied_items);
    } else {
        println!("last_applied_backend: none");
        println!("last_applied_items: 0");
    }
    Ok(())
}

fn export_grub_theme_command(args: Vec<String>) -> Result<(), String> {
    let config_path = config_path_from_args(&args);
    let config = AppConfig::load_or_default(&config_path).map_err(|error| error.to_string())?;
    let theme_dir = optional_flag(&args, "--theme-dir")
        .map(PathBuf::from)
        .unwrap_or_else(default_grub_theme_dir);
    let background = optional_flag(&args, "--background").map(PathBuf::from);
    let title = optional_flag(&args, "--title").unwrap_or_else(|| "aurora-wall".to_string());

    let results = export_grub_theme(&config, &theme_dir, background.as_deref(), &title)
        .map_err(|error| error.to_string())?;

    for line in results {
        println!("{}", line);
    }

    println!(
        "next: review {}/INSTALL.txt and run the GRUB install commands with sudo",
        theme_dir.display()
    );
    Ok(())
}

fn export_video_poster_command(args: Vec<String>) -> Result<(), String> {
    let video = required_flag(&args, "--video")?;
    let output = optional_flag(&args, "--output")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/aurora-wall-poster.png"));
    let timestamp = optional_flag(&args, "--at").unwrap_or_else(|| "00:00:03".to_string());

    let poster =
        export_video_poster(PathBuf::from(video).as_path(), &output, &timestamp).map_err(|error| error.to_string())?;
    println!("wrote {}", poster.display());
    Ok(())
}

fn export_boot_theme_command(args: Vec<String>) -> Result<(), String> {
    let config_path = config_path_from_args(&args);
    let config = AppConfig::load_or_default(&config_path).map_err(|error| error.to_string())?;
    let video = PathBuf::from(required_flag(&args, "--video")?);
    let title = optional_flag(&args, "--title").unwrap_or_else(|| "aurora-wall".to_string());
    let timestamp = optional_flag(&args, "--at").unwrap_or_else(|| "00:00:03".to_string());
    let poster = optional_flag(&args, "--poster")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/aurora-wall-poster.png"));
    let grub_dir = optional_flag(&args, "--grub-dir")
        .map(PathBuf::from)
        .unwrap_or_else(default_grub_theme_dir);
    let plymouth_dir = optional_flag(&args, "--plymouth-dir")
        .map(PathBuf::from)
        .unwrap_or_else(default_plymouth_theme_dir);

    let poster_path =
        export_video_poster(&video, &poster, &timestamp).map_err(|error| error.to_string())?;
    println!("wrote {}", poster_path.display());

    let grub_results = export_grub_theme(&config, &grub_dir, Some(&poster_path), &title)
        .map_err(|error| error.to_string())?;
    for line in grub_results {
        println!("{}", line);
    }

    let plymouth_results = export_plymouth_theme(
        &config,
        &plymouth_dir,
        Some(&poster_path),
        Some(&video),
        &title,
        &timestamp,
    )
    .map_err(|error| error.to_string())?;
    for line in plymouth_results {
        println!("{}", line);
    }

    println!("next: review {}/INSTALL.txt", grub_dir.display());
    println!("next: review {}/INSTALL.txt", plymouth_dir.display());
    Ok(())
}

fn install_boot_theme_command(args: Vec<String>) -> Result<(), String> {
    let grub_dir = optional_flag(&args, "--grub-dir")
        .map(PathBuf::from)
        .unwrap_or_else(default_grub_theme_dir);
    let plymouth_dir = optional_flag(&args, "--plymouth-dir")
        .map(PathBuf::from)
        .unwrap_or_else(default_plymouth_theme_dir);

    let results = install_boot_theme(&grub_dir, &plymouth_dir).map_err(|error| {
        if error.kind() == std::io::ErrorKind::PermissionDenied {
            format!(
                "{}. Re-run this command with sudo.",
                error
            )
        } else {
            error.to_string()
        }
    })?;

    for line in results {
        println!("{}", line);
    }

    Ok(())
}

fn export_plymouth_theme_command(args: Vec<String>) -> Result<(), String> {
    let config_path = config_path_from_args(&args);
    let config = AppConfig::load_or_default(&config_path).map_err(|error| error.to_string())?;
    let theme_dir = optional_flag(&args, "--theme-dir")
        .map(PathBuf::from)
        .unwrap_or_else(default_plymouth_theme_dir);
    let background = optional_flag(&args, "--background").map(PathBuf::from);
    let video = optional_flag(&args, "--video").map(PathBuf::from);
    let title = optional_flag(&args, "--title").unwrap_or_else(|| "aurora-wall".to_string());
    let timestamp = optional_flag(&args, "--at").unwrap_or_else(|| "00:00:03".to_string());

    let results = export_plymouth_theme(
        &config,
        &theme_dir,
        background.as_deref(),
        video.as_deref(),
        &title,
        &timestamp,
    )
    .map_err(|error| error.to_string())?;

    for line in results {
        println!("{}", line);
    }

    println!(
        "next: review {}/INSTALL.txt and install the Plymouth theme with sudo",
        theme_dir.display()
    );
    Ok(())
}

fn config_path_from_args(args: &[String]) -> PathBuf {
    optional_flag(args, "--config")
        .map(PathBuf::from)
        .unwrap_or_else(default_config_path)
}

fn required_flag(args: &[String], flag: &str) -> Result<String, String> {
    optional_flag(args, flag).ok_or_else(|| format!("missing required flag: {}", flag))
}

fn optional_flag(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].clone())
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn parse_yes_no(input: &str) -> Result<bool, String> {
    match input.trim().to_ascii_lowercase().as_str() {
        "yes" | "true" | "1" => Ok(true),
        "no" | "false" | "0" => Ok(false),
        _ => Err(format!("invalid yes/no value: {}", input)),
    }
}

fn validate_output_if_possible(output: &str) -> Result<(), String> {
    match list_outputs() {
        Ok(outputs) if !outputs.is_empty() && !outputs.iter().any(|item| item == output) => Err(
            format!(
                "unknown output: {}. Available outputs: {}",
                output,
                outputs.join(", ")
            ),
        ),
        _ => Ok(()),
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}
