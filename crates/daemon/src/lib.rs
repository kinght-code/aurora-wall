use aurora_wall_backend_api::{WallpaperBackend, WallpaperKind, WallpaperSpec};
use aurora_wall_backend_hyprland::{list_monitors, HyprlandBackend, HyprlandEnvironment};
use aurora_wall_config::{default_config_path, AppConfig};
use aurora_wall_state::{default_state_path, AppliedState, RestorePolicy};
use std::fs;
use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonStatus {
    phase: String,
}

impl DaemonStatus {
    pub fn planned() -> Self {
        Self {
            phase: "ready".to_string(),
        }
    }

    pub fn summary(&self) -> &str {
        &self.phase
    }

    pub fn boot_summary(config: &AppConfig) -> String {
        format!(
            "backend={}, target_family={}, restore_on_login={}, restore_policy={}",
            HyprlandBackend.kind().as_str(),
            config.target_family,
            config.restore_on_login,
            RestorePolicy::default().as_str()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyStatus {
    pub hyprctl: bool,
    pub swww: bool,
    pub awww: bool,
    pub mpvpaper: bool,
    pub wal: bool,
    pub pkill: bool,
}

impl DependencyStatus {
    pub fn detect() -> Self {
        Self {
            hyprctl: command_exists("hyprctl"),
            swww: command_exists("swww"),
            awww: command_exists("awww"),
            mpvpaper: command_exists("mpvpaper"),
            wal: command_exists("wal"),
            pkill: command_exists("pkill"),
        }
    }

    pub fn summary_lines(&self) -> Vec<String> {
        vec![
            format!("hyprctl={}", yes_no(self.hyprctl)),
            format!("swww={}", yes_no(self.swww)),
            format!("awww={}", yes_no(self.awww)),
            format!("mpvpaper={}", yes_no(self.mpvpaper)),
            format!("wal={}", yes_no(self.wal)),
            format!("pkill={}", yes_no(self.pkill)),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStatus {
    pub config_path: PathBuf,
    pub state_path: PathBuf,
    pub environment: HyprlandEnvironment,
    pub dependencies: DependencyStatus,
    pub configured_wallpapers: usize,
}

impl RuntimeStatus {
    pub fn load(config_path: Option<&Path>) -> io::Result<Self> {
        let config_path = config_path
            .map(Path::to_path_buf)
            .unwrap_or_else(default_config_path);
        let config = AppConfig::load_or_default(&config_path)?;

        Ok(Self {
            config_path,
            state_path: default_state_path(),
            environment: HyprlandEnvironment::detect(),
            dependencies: DependencyStatus::detect(),
            configured_wallpapers: config.wallpapers.len(),
        })
    }
}

pub fn ensure_library(config: &AppConfig) -> io::Result<()> {
    std::fs::create_dir_all(&config.library_dir)
}

pub fn write_default_config(path: &Path) -> io::Result<AppConfig> {
    let config = AppConfig::default();
    config.save(path)?;
    Ok(config)
}

pub fn list_outputs() -> io::Result<Vec<String>> {
    match list_monitors() {
        Ok(monitors) if !monitors.is_empty() => {
            Ok(monitors.into_iter().map(|monitor| monitor.name).collect())
        }
        _ => list_outputs_via_awww(),
    }
}

pub fn upsert_wallpaper(config: &mut AppConfig, spec: WallpaperSpec) {
    if let Some(existing) = config
        .wallpapers
        .iter_mut()
        .find(|wallpaper| wallpaper.output == spec.output)
    {
        *existing = spec;
    } else {
        config.wallpapers.push(spec);
    }
}

pub fn apply_config(config: &AppConfig, verbose: bool) -> io::Result<Vec<String>> {
    ensure_library(config)?;
    let mut actions = Vec::new();
    let available_outputs = list_outputs().unwrap_or_default();

    for wallpaper in &config.wallpapers {
        wallpaper
            .validate()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;

        if !available_outputs.is_empty() && !available_outputs.iter().any(|output| output == &wallpaper.output) {
            if verbose {
                actions.push(format!(
                    "skipped unavailable output {} for {} wallpaper",
                    wallpaper.output,
                    wallpaper.kind.as_str()
                ));
            }
            continue;
        }

        let command_line = match wallpaper.kind {
            WallpaperKind::Image => apply_image(wallpaper, verbose)?,
            WallpaperKind::Video => apply_video(wallpaper, verbose)?,
        };
        if verbose {
            actions.push(command_line);
        }
    }

    let state = AppliedState::new(HyprlandBackend.kind().as_str(), config.wallpapers.len());
    state.save(&default_state_path())?;
    Ok(actions)
}

fn apply_image(spec: &WallpaperSpec, _verbose: bool) -> io::Result<String> {
    if command_exists("swww") {
        let _ = Command::new("swww-daemon").spawn();
        let transition = if spec.transition.as_str() == "fade" {
            vec!["--transition-type", "fade"]
        } else {
            vec!["--transition-type", "none"]
        };

        let status = Command::new("swww")
            .arg("img")
            .arg(&spec.path)
            .arg("--outputs")
            .arg(&spec.output)
            .arg("--resize")
            .arg(spec.scaling.as_str())
            .args(transition)
            .status()?;

        if !status.success() {
            return Err(io::Error::other(format!(
                "swww failed for output {}",
                spec.output
            )));
        }

        return Ok(format!(
            "swww img {} --outputs {} --resize {} --transition-type {}",
            spec.path,
            spec.output,
            spec.scaling.as_str(),
            spec.transition.as_str()
        ));
    }

    if !command_exists("awww") {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "swww or awww is required for image wallpapers. Install one with: sudo pacman -S swww",
        ));
    }

    let _ = Command::new("awww-daemon").spawn();
    let resize = match spec.scaling.as_str() {
        "fill" => "crop",
        "fit" => "fit",
        "center" => "no",
        _ => "crop",
    };
    let transition = if spec.transition.as_str() == "fade" {
        "fade"
    } else {
        "none"
    };

    let status = Command::new("awww")
        .arg("img")
        .arg(&spec.path)
        .arg("--outputs")
        .arg(&spec.output)
        .arg("--resize")
        .arg(resize)
        .arg("--transition-type")
        .arg(transition)
        .status()?;

    if !status.success() {
        return Err(io::Error::other(format!(
            "awww failed for output {}",
            spec.output
        )));
    }

    Ok(format!(
        "awww img {} --outputs {} --resize {} --transition-type {}",
        spec.path,
        spec.output,
        resize,
        transition
    ))
}

fn apply_video(spec: &WallpaperSpec, verbose: bool) -> io::Result<String> {
    if !command_exists("mpvpaper") {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "mpvpaper is required for live wallpapers. Install it with: sudo pacman -S mpvpaper mpv",
        ));
    }

    if command_exists("pkill") {
        let _ = Command::new("pkill")
            .args(["-f", &format!("mpvpaper {}", spec.output)])
            .status();
    }

    let mut options = vec!["no-audio".to_string()];
    if !spec.muted {
        options.clear();
    }

    options.push(match spec.loop_mode {
        aurora_wall_backend_api::LoopMode::Infinite => "loop-playlist=inf".to_string(),
        aurora_wall_backend_api::LoopMode::Once => "loop-file=no".to_string(),
    });

    let options_text = options.join(" ");
    let mut command = Command::new("mpvpaper");
    command
        .arg("-o")
        .arg(&options_text)
        .arg(&spec.output)
        .arg(&spec.path)
        .stdin(Stdio::null());

    if verbose {
        command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    } else {
        command.stdout(Stdio::null()).stderr(Stdio::null());
    }

    command.spawn()?;

    Ok(format!(
        "mpvpaper -o \"{}\" {} {}",
        options_text,
        spec.output,
        spec.path
    ))
}

pub fn arch_install_hint() -> &'static str {
    "sudo pacman -S swww mpvpaper mpv  # or install awww instead of swww for still images"
}

pub fn systemd_user_service() -> String {
    format!(
        "[Unit]\nDescription=aurora-wall restore service\nAfter=graphical-session.target\n\n[Service]\nType=oneshot\nExecStart={} apply --restore\n\n[Install]\nWantedBy=default.target\n",
        current_binary_hint()
    )
}

pub fn pkgbuild_template() -> &'static str {
    "pkgname=aurora-wall\npkgver=0.1.0\npkgrel=1\npkgdesc=\"Hyprland-first wallpaper manager for still and live wallpapers\"\narch=('x86_64')\ndepends=('swww' 'mpvpaper' 'mpv')\nmakedepends=('cargo')\nsource=()\nsha256sums=()\n"
}

pub fn export_grub_theme(
    config: &AppConfig,
    theme_dir: &Path,
    requested_background: Option<&Path>,
    title: &str,
) -> io::Result<Vec<String>> {
    fs::create_dir_all(theme_dir)?;

    let background = resolve_grub_background(config, requested_background)?;
    let extension = background
        .extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| !ext.is_empty())
        .unwrap_or("png");
    let background_name = format!("background.{}", extension);
    let target_background = theme_dir.join(&background_name);
    fs::copy(&background, &target_background)?;

    let theme_text = format!(
        "+ boot_menu {{\n  left = 8%\n  top = 20%\n  width = 42%\n  height = 52%\n  item_color = \"#f5f5f5\"\n  selected_item_color = \"#111111\"\n  selected_item_pixmap_style = \"menuitem_select\"\n  item_spacing = 10\n  item_height = 36\n  item_padding = 8\n}}\n\n+ label {{\n  id = \"title\"\n  left = 8%\n  top = 10%\n  width = 60%\n  height = 40\n  text = \"{}\"\n  color = \"#f5f5f5\"\n  align = \"left\"\n  font = \"Unifont Regular 20\"\n}}\n\n+ image {{\n  file = \"{}\"\n  left = 0\n  top = 0\n  width = 100%\n  height = 100%\n}}\n\n+ rect {{\n  left = 5%\n  top = 8%\n  width = 50%\n  height = 70%\n  color = \"#000000\"\n  alpha = 96\n}}\n\n+ rect {{\n  id = \"menuitem_select\"\n  left = 0\n  top = 0\n  width = 100%\n  height = 100%\n  color = \"#f5f5f5\"\n  alpha = 210\n}}\n",
        sanitize_grub_text(title),
        background_name
    );
    fs::write(theme_dir.join("theme.txt"), theme_text)?;

    let install_text = format!(
        "Copy this theme to GRUB and set it as active:\n\nsudo mkdir -p /boot/grub/themes/aurora-wall\nsudo cp -r \"{}\"/* /boot/grub/themes/aurora-wall/\necho 'GRUB_THEME=/boot/grub/themes/aurora-wall/theme.txt' | sudo tee /etc/default/grub.d/aurora-wall.cfg\nsudo grub-mkconfig -o /boot/grub/grub.cfg\n",
        theme_dir.display()
    );
    fs::write(theme_dir.join("INSTALL.txt"), install_text)?;

    Ok(vec![
        format!("wrote {}", theme_dir.join("theme.txt").display()),
        format!("copied background {}", target_background.display()),
        format!("wrote {}", theme_dir.join("INSTALL.txt").display()),
    ])
}

pub fn default_grub_theme_dir() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local/share/aurora-wall/grub-theme")
}

pub fn export_video_poster(video_path: &Path, output_path: &Path, timestamp: &str) -> io::Result<PathBuf> {
    if !video_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("video does not exist: {}", video_path.display()),
        ));
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let status = Command::new("ffmpeg")
        .args(["-y", "-ss", timestamp, "-i"])
        .arg(video_path)
        .args(["-frames:v", "1"])
        .arg(output_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    if !status.success() {
        return Err(io::Error::other(format!(
            "ffmpeg failed to extract poster frame from {}",
            video_path.display()
        )));
    }

    Ok(output_path.to_path_buf())
}

pub fn sync_wal_theme(config: &AppConfig, verbose: bool) -> io::Result<Option<String>> {
    if !command_exists("wal") {
        return Ok(None);
    }

    let source = if let Some(image_wallpaper) = config
        .wallpapers
        .iter()
        .find(|wallpaper| wallpaper.kind == WallpaperKind::Image)
    {
        PathBuf::from(&image_wallpaper.path)
    } else if let Some(video_wallpaper) = config
        .wallpapers
        .iter()
        .find(|wallpaper| wallpaper.kind == WallpaperKind::Video)
    {
        export_video_poster(
            Path::new(&video_wallpaper.path),
            &PathBuf::from("/tmp/aurora-wall-wal-poster.png"),
            "00:00:03",
        )?
    } else {
        return Ok(None);
    };

    let mut command = Command::new("wal");
    command.arg("-i").arg(&source).stdin(Stdio::null());

    if verbose {
        command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    } else {
        command.stdout(Stdio::null()).stderr(Stdio::null());
    }

    let status = command.status()?;
    if !status.success() {
        return Err(io::Error::other(format!(
            "wal failed while syncing theme from {}",
            source.display()
        )));
    }

    Ok(Some(format!("wal synced from {}", source.display())))
}

pub fn export_plymouth_theme(
    config: &AppConfig,
    theme_dir: &Path,
    requested_background: Option<&Path>,
    requested_video: Option<&Path>,
    title: &str,
    timestamp: &str,
) -> io::Result<Vec<String>> {
    fs::create_dir_all(theme_dir)?;

    let background = if let Some(background) = requested_background {
        background.to_path_buf()
    } else if let Some(video) = requested_video {
        export_video_poster(video, &theme_dir.join("poster.png"), timestamp)?
    } else if let Some(image_wallpaper) = config
        .wallpapers
        .iter()
        .find(|wallpaper| wallpaper.kind == WallpaperKind::Image)
    {
        PathBuf::from(&image_wallpaper.path)
    } else if let Some(video_wallpaper) = config
        .wallpapers
        .iter()
        .find(|wallpaper| wallpaper.kind == WallpaperKind::Video)
    {
        export_video_poster(
            Path::new(&video_wallpaper.path),
            &theme_dir.join("poster.png"),
            timestamp,
        )?
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no suitable wallpaper found for Plymouth export; pass --background or --video",
        ));
    };

    let background_target = copy_with_name(&background, &theme_dir.join("background.png"))?;
    let palette = derive_palette(&background_target)?;
    let theme_name = "aurora-wall";
    let plymouth_file = theme_dir.join(format!("{}.plymouth", theme_name));
    let script_file = theme_dir.join(format!("{}.script", theme_name));

    let plymouth_text = format!(
        "[Plymouth Theme]\nName={}\nDescription=Aurora Wall Plymouth theme\nModuleName=script\n\n[script]\nImageDir={}\nScriptFile={}\n",
        sanitize_grub_text(title),
        theme_dir.display(),
        script_file.display()
    );
    fs::write(&plymouth_file, plymouth_text)?;

    let script_text = format!(
        "screen_width = Window.GetWidth();\nscreen_height = Window.GetHeight();\n\nbg_image = Image(\"background.png\");\nbg_sprite = Sprite(bg_image);\nbg_sprite.SetZ(-100);\nbg_sprite.SetPosition(screen_width / 2, screen_height / 2);\n\nlogo_box = Box();\nlogo_box.SetColorTop({}, {}, {}, 0.72);\nlogo_box.SetColorBottom({}, {}, {}, 0.72);\nlogo_box.SetPosition(screen_width * 0.08, screen_height * 0.10, screen_width * 0.42, screen_height * 0.18);\n\nmessage = \"{}\";\nmessage_image = Image.Text(message, {}, {}, {}, 1);\nmessage_sprite = Sprite(message_image);\nmessage_sprite.SetZ(100);\nmessage_sprite.SetPosition(screen_width * 0.11, screen_height * 0.16);\n\nfun refresh_callback () {{\n  global logo_box;\n  global message_sprite;\n  logo_box.SetColorTop({}, {}, {}, 0.72);\n  message_sprite.SetOpacity(1);\n}}\n\nPlymouth.SetRefreshFunction(refresh_callback);\n",
        palette.overlay_r,
        palette.overlay_g,
        palette.overlay_b,
        palette.shadow_r,
        palette.shadow_g,
        palette.shadow_b,
        sanitize_grub_text(title),
        palette.text_r,
        palette.text_g,
        palette.text_b,
        palette.overlay_r,
        palette.overlay_g,
        palette.overlay_b
    );
    fs::write(&script_file, script_text)?;

    let install_text = format!(
        "Install Plymouth and this theme:\n\nsudo pacman -S plymouth\nsudo mkdir -p /usr/share/plymouth/themes/{name}\nsudo cp -r \"{dir}\"/* /usr/share/plymouth/themes/{name}/\nsudo plymouth-set-default-theme -R {name}\n\nIf your distro does not have plymouth-set-default-theme, set the theme in your initramfs config and rebuild it manually.\n",
        name = theme_name,
        dir = theme_dir.display()
    );
    fs::write(theme_dir.join("INSTALL.txt"), install_text)?;

    Ok(vec![
        format!("wrote {}", plymouth_file.display()),
        format!("wrote {}", script_file.display()),
        format!("copied background {}", background_target.display()),
        format!("wrote {}", theme_dir.join("INSTALL.txt").display()),
    ])
}

pub fn default_plymouth_theme_dir() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local/share/aurora-wall/plymouth-theme")
}

pub fn install_boot_theme(grub_source: &Path, plymouth_source: &Path) -> io::Result<Vec<String>> {
    if !grub_source.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("GRUB theme directory does not exist: {}", grub_source.display()),
        ));
    }
    if !plymouth_source.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Plymouth theme directory does not exist: {}",
                plymouth_source.display()
            ),
        ));
    }

    let grub_target = PathBuf::from("/boot/grub/themes/aurora-wall");
    let grub_default = PathBuf::from("/etc/default/grub.d/aurora-wall.cfg");
    let plymouth_target = PathBuf::from("/usr/share/plymouth/themes/aurora-wall");

    copy_dir_recursive(grub_source, &grub_target)?;
    if let Some(parent) = grub_default.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &grub_default,
        "GRUB_THEME=/boot/grub/themes/aurora-wall/theme.txt\n",
    )?;

    copy_dir_recursive(plymouth_source, &plymouth_target)?;

    if command_exists("plymouth-set-default-theme") {
        let status = Command::new("plymouth-set-default-theme")
            .args(["-R", "aurora-wall"])
            .status()?;
        if !status.success() {
            return Err(io::Error::other(
                "plymouth-set-default-theme failed while activating aurora-wall",
            ));
        }
    }

    if command_exists("grub-mkconfig") {
        let status = Command::new("grub-mkconfig")
            .args(["-o", "/boot/grub/grub.cfg"])
            .status()?;
        if !status.success() {
            return Err(io::Error::other(
                "grub-mkconfig failed while rebuilding /boot/grub/grub.cfg",
            ));
        }
    }

    Ok(vec![
        format!("installed GRUB theme to {}", grub_target.display()),
        format!("wrote {}", grub_default.display()),
        format!("installed Plymouth theme to {}", plymouth_target.display()),
        "rebuilt boot configuration where supported".to_string(),
    ])
}

fn current_binary_hint() -> String {
    env::current_exe()
        .ok()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "aurora-wall".to_string())
}

fn resolve_grub_background(
    config: &AppConfig,
    requested_background: Option<&Path>,
) -> io::Result<PathBuf> {
    if let Some(path) = requested_background {
        if path.exists() {
            return Ok(path.to_path_buf());
        }

        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("GRUB background does not exist: {}", path.display()),
        ));
    }

    if let Some(wallpaper) = config
        .wallpapers
        .iter()
        .find(|wallpaper| wallpaper.kind == WallpaperKind::Image)
    {
        return Ok(PathBuf::from(&wallpaper.path));
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "GRUB needs a static background image. Your config only has video wallpapers, so pass --background /path/to/image.jpg",
    ))
}

fn sanitize_grub_text(input: &str) -> String {
    input.replace('"', "'")
}

fn copy_with_name(source: &Path, target: &Path) -> io::Result<PathBuf> {
    if !source.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("background does not exist: {}", source.display()),
        ));
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(source, target)?;
    Ok(target.to_path_buf())
}

fn copy_dir_recursive(source: &Path, target: &Path) -> io::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &target_path)?;
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct ThemePalette {
    overlay_r: u8,
    overlay_g: u8,
    overlay_b: u8,
    shadow_r: u8,
    shadow_g: u8,
    shadow_b: u8,
    text_r: u8,
    text_g: u8,
    text_b: u8,
}

fn derive_palette(image_path: &Path) -> io::Result<ThemePalette> {
    let output = Command::new("magick")
        .arg(image_path)
        .args(["-resize", "1x1!", "-format", "%[pixel:p{0,0}]", "info:"])
        .output()?;

    if !output.status.success() {
        return Err(io::Error::other(format!(
            "failed to derive palette from {}",
            image_path.display()
        )));
    }

    let pixel = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let (r, g, b) = parse_magick_pixel(&pixel)?;
    let luminance = (0.2126 * f32::from(r) + 0.7152 * f32::from(g) + 0.0722 * f32::from(b)) / 255.0;

    let (text_r, text_g, text_b) = if luminance > 0.5 {
        (20, 20, 24)
    } else {
        (245, 245, 245)
    };

    Ok(ThemePalette {
        overlay_r: darken(r, 0.35),
        overlay_g: darken(g, 0.35),
        overlay_b: darken(b, 0.35),
        shadow_r: darken(r, 0.55),
        shadow_g: darken(g, 0.55),
        shadow_b: darken(b, 0.55),
        text_r,
        text_g,
        text_b,
    })
}

fn parse_magick_pixel(pixel: &str) -> io::Result<(u8, u8, u8)> {
    let Some(inner) = pixel
        .strip_prefix("srgb(")
        .and_then(|value| value.strip_suffix(')'))
    else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported pixel format: {}", pixel),
        ));
    };

    let parts = inner.split(',').map(|item| item.trim()).collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid pixel value: {}", pixel),
        ));
    }

    let r = parts[0]
        .trim_end_matches('%');
    let g = parts[1]
        .trim_end_matches('%');
    let b = parts[2]
        .trim_end_matches('%');

    Ok((
        parse_color_channel(parts[0], r)?,
        parse_color_channel(parts[1], g)?,
        parse_color_channel(parts[2], b)?,
    ))
}

fn darken(channel: u8, factor: f32) -> u8 {
    ((f32::from(channel) * factor).round() as i32).clamp(0, 255) as u8
}

fn parse_color_channel(raw: &str, stripped: &str) -> io::Result<u8> {
    if raw.contains('%') {
        let percent = stripped.parse::<f32>().map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, format!("invalid percent channel: {}", raw))
        })?;
        return Ok(((percent / 100.0) * 255.0).round().clamp(0.0, 255.0) as u8);
    }

    stripped.parse::<u8>().map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidData, format!("invalid integer channel: {}", raw))
    })
}

fn command_exists(command: &str) -> bool {
    env::var_os("PATH")
        .map(|paths| env::split_paths(&paths).any(|path| path.join(command).exists()))
        .unwrap_or(false)
}

fn list_outputs_via_awww() -> io::Result<Vec<String>> {
    if !command_exists("awww") {
        return Err(io::Error::other("no output discovery backend available"));
    }

    let output = Command::new("awww").arg("query").output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(stderr.trim().to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let outputs = stdout
        .lines()
        .filter_map(|line| {
            line.split_once(':')
                .map(|(name, _)| name.trim().to_string())
                .filter(|name| !name.is_empty())
        })
        .collect::<Vec<_>>();

    Ok(outputs)
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}
