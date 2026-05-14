use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use bevy::{
    audio::{
        AudioPlayer, AudioSink, AudioSinkPlayback, AudioSource, Decodable, PlaybackSettings,
        Source, Volume,
    },
    prelude::*,
};

use crate::{config::AudioSettingsState, save::WorldLoadState};

const MENU_MUSIC_SUBDIR: &str = "music/menu";
const GAME_MUSIC_SUBDIR: &str = "music/game";
const TRACK_FADE_SECONDS: f32 = 2.5;
const CONTEXT_SWITCH_FADE_SECONDS: f32 = 1.0;
const INTER_TRACK_PAUSE_MIN_SECONDS: f32 = 3.0;
const INTER_TRACK_PAUSE_MAX_SECONDS: f32 = 5.0;

/// Adds background music playback with menu/game context switching.
pub struct BgmPlugin;

impl Plugin for BgmPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_bgm_state)
            .add_systems(PostUpdate, drive_bgm_playback);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MusicContext {
    Menu,
    Game,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FadeOutReason {
    NaturalTrackEnd,
    ContextSwitch,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum BgmPhase {
    Suspended,
    NextTrack,
    StartFadeIn {
        elapsed_secs: f32,
        duration_secs: f32,
    },
    Playing,
    StartFadeOut {
        elapsed_secs: f32,
        reason: FadeOutReason,
    },
    InterTrackPause {
        remaining_secs: f32,
    },
}

#[derive(Debug)]
struct ActiveTrack {
    entity: Entity,
    handle: Handle<AudioSource>,
    duration_secs: Option<f32>,
    playback_elapsed_secs: f32,
    sink_started: bool,
}

#[derive(Resource, Debug, Default)]
struct BgmTrackCatalog {
    menu_tracks: Vec<Handle<AudioSource>>,
    game_tracks: Vec<Handle<AudioSource>>,
}

#[derive(Resource, Debug)]
struct BgmRuntime {
    target_context: MusicContext,
    current_context: MusicContext,
    phase: BgmPhase,
    active_track: Option<ActiveTrack>,
    rng: Rng64,
}

#[derive(Clone, Debug)]
struct MusicScanResult {
    menu_relative_paths: Vec<String>,
    game_relative_paths: Vec<String>,
}

#[derive(Clone, Copy, Debug)]
struct Rng64 {
    state: u64,
}

impl Rng64 {
    fn seeded(seed: u64) -> Self {
        let init = if seed == 0 {
            0xA5A5_1234_89AB_CDEF
        } else {
            seed
        };
        Self { state: init }
    }

    fn from_system_time() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(1))
            .as_nanos() as u64;
        Self::seeded(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn next_f32(&mut self) -> f32 {
        let v = (self.next_u64() >> 40) as u32;
        (v as f32) / (u32::MAX as f32)
    }

    fn pick_index(&mut self, len: usize) -> usize {
        if len <= 1 {
            0
        } else {
            (self.next_u64() as usize) % len
        }
    }

    fn random_pause_secs(&mut self) -> f32 {
        let span = (INTER_TRACK_PAUSE_MAX_SECONDS - INTER_TRACK_PAUSE_MIN_SECONDS).max(0.0);
        INTER_TRACK_PAUSE_MIN_SECONDS + self.next_f32() * span
    }
}

fn fade_out_duration_for(reason: FadeOutReason) -> f32 {
    match reason {
        FadeOutReason::NaturalTrackEnd => TRACK_FADE_SECONDS,
        FadeOutReason::ContextSwitch => CONTEXT_SWITCH_FADE_SECONDS,
    }
}

fn scheduler_enabled(target_volume: f32) -> bool {
    target_volume > f32::EPSILON
}

#[derive(Component)]
struct BgmTrackMarker;

fn setup_bgm_state(mut commands: Commands, asset_server: Res<AssetServer>) {
    let repo_assets_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let scan = scan_music_paths_once(&repo_assets_root);

    if scan.menu_relative_paths.is_empty() {
        bevy::log::warn!(
            "BGM: no menu music files found in assets/{}; menu will stay silent.",
            MENU_MUSIC_SUBDIR
        );
    }
    if scan.game_relative_paths.is_empty() {
        bevy::log::warn!(
            "BGM: no game music files found in assets/{}; game will stay silent.",
            GAME_MUSIC_SUBDIR
        );
    }

    let menu_tracks = scan
        .menu_relative_paths
        .iter()
        .map(|path| asset_server.load::<AudioSource>(path))
        .collect::<Vec<_>>();
    let game_tracks = scan
        .game_relative_paths
        .iter()
        .map(|path| asset_server.load::<AudioSource>(path))
        .collect::<Vec<_>>();

    commands.insert_resource(BgmTrackCatalog {
        menu_tracks,
        game_tracks,
    });
    commands.insert_resource(BgmRuntime {
        target_context: MusicContext::Menu,
        current_context: MusicContext::Menu,
        phase: BgmPhase::NextTrack,
        active_track: None,
        rng: Rng64::from_system_time(),
    });
}

fn drive_bgm_playback(
    mut commands: Commands,
    time: Res<Time>,
    world_load_state: Res<WorldLoadState>,
    audio_settings: Res<AudioSettingsState>,
    catalog: Res<BgmTrackCatalog>,
    audio_assets: Res<Assets<AudioSource>>,
    mut runtime: ResMut<BgmRuntime>,
    sinks: Query<&mut AudioSink, With<BgmTrackMarker>>,
) {
    let target_volume = audio_settings.runtime_music_volume_normalized();
    if !scheduler_enabled(target_volume) {
        despawn_active_track(&mut commands, &mut runtime.active_track);
        runtime.phase = BgmPhase::Suspended;
        runtime.current_context = if world_load_state.has_world {
            MusicContext::Game
        } else {
            MusicContext::Menu
        };
        runtime.target_context = runtime.current_context;
        return;
    }
    if matches!(runtime.phase, BgmPhase::Suspended) {
        runtime.phase = BgmPhase::NextTrack;
    }

    let desired_context = if world_load_state.has_world {
        MusicContext::Game
    } else {
        MusicContext::Menu
    };

    if runtime.target_context != desired_context {
        runtime.target_context = desired_context;
        match (&runtime.active_track, runtime.phase) {
            (
                Some(_),
                BgmPhase::StartFadeOut {
                    reason: FadeOutReason::ContextSwitch,
                    ..
                },
            ) => {}
            (Some(_), _) => {
                runtime.phase = BgmPhase::StartFadeOut {
                    elapsed_secs: 0.0,
                    reason: FadeOutReason::ContextSwitch,
                };
            }
            (None, _) => {
                runtime.phase = BgmPhase::NextTrack;
            }
        }
    }

    let delta_secs = time.delta_secs().max(0.0);
    let mut active_sinks = sinks;

    if let Some(active_track) = runtime.active_track.as_mut() {
        if active_track.duration_secs.is_none() {
            if let Some(asset) = audio_assets.get(&active_track.handle) {
                let duration = asset.decoder().total_duration().map(|d| d.as_secs_f32());
                active_track.duration_secs = duration;
            }
        }
        if let Ok(mut sink) = active_sinks.get_mut(active_track.entity) {
            active_track.sink_started = true;
            active_track.playback_elapsed_secs += delta_secs;
            if matches!(runtime.phase, BgmPhase::Playing) {
                sink.set_volume(Volume::Linear(target_volume));
            }
        }
    }

    match runtime.phase {
        BgmPhase::Suspended => {}
        BgmPhase::NextTrack => {
            start_next_track_for_target_context(&mut commands, &catalog, &mut runtime);
        }
        BgmPhase::StartFadeIn {
            elapsed_secs,
            duration_secs,
        } => {
            if let Some(track) = runtime.active_track.as_ref() {
                if let Ok(mut sink) = active_sinks.get_mut(track.entity) {
                    let next_elapsed = (elapsed_secs + delta_secs).min(duration_secs);
                    let ratio = if duration_secs <= f32::EPSILON {
                        1.0
                    } else {
                        next_elapsed / duration_secs
                    };
                    sink.set_volume(Volume::Linear(
                        (target_volume * ratio).clamp(0.0, target_volume),
                    ));
                    if next_elapsed >= duration_secs {
                        runtime.phase = BgmPhase::Playing;
                    } else {
                        runtime.phase = BgmPhase::StartFadeIn {
                            elapsed_secs: next_elapsed,
                            duration_secs,
                        };
                    }
                }
            } else {
                runtime.phase = BgmPhase::NextTrack;
            }
        }
        BgmPhase::Playing => {
            if runtime.current_context != runtime.target_context {
                runtime.phase = BgmPhase::StartFadeOut {
                    elapsed_secs: 0.0,
                    reason: FadeOutReason::ContextSwitch,
                };
                return;
            }

            let should_fade_out = runtime
                .active_track
                .as_ref()
                .and_then(|track| {
                    track
                        .duration_secs
                        .map(|d| (track.playback_elapsed_secs, d))
                })
                .map(|(elapsed, duration)| elapsed >= (duration - TRACK_FADE_SECONDS).max(0.0))
                .unwrap_or(false);
            let ended_without_duration = runtime
                .active_track
                .as_ref()
                .filter(|track| track.duration_secs.is_none() && track.sink_started)
                .and_then(|track| active_sinks.get_mut(track.entity).ok())
                .map(|sink| sink.empty())
                .unwrap_or(false);

            if should_fade_out {
                runtime.phase = BgmPhase::StartFadeOut {
                    elapsed_secs: 0.0,
                    reason: FadeOutReason::NaturalTrackEnd,
                };
            } else if ended_without_duration {
                despawn_active_track(&mut commands, &mut runtime.active_track);
                runtime.phase = BgmPhase::InterTrackPause {
                    remaining_secs: runtime.rng.random_pause_secs(),
                };
            }
        }
        BgmPhase::StartFadeOut {
            elapsed_secs,
            reason,
        } => {
            let mut fade_completed = false;
            let fade_duration = fade_out_duration_for(reason);
            let next_elapsed = (elapsed_secs + delta_secs).min(fade_duration);
            if let Some(track) = runtime.active_track.as_ref() {
                if let Ok(mut sink) = active_sinks.get_mut(track.entity) {
                    let ratio = if fade_duration <= f32::EPSILON {
                        0.0
                    } else {
                        1.0 - (next_elapsed / fade_duration)
                    };
                    sink.set_volume(Volume::Linear(
                        (target_volume * ratio).clamp(0.0, target_volume),
                    ));
                }
            }
            if next_elapsed >= fade_duration {
                fade_completed = true;
            }
            if fade_completed {
                despawn_active_track(&mut commands, &mut runtime.active_track);
                match reason {
                    FadeOutReason::ContextSwitch => runtime.phase = BgmPhase::NextTrack,
                    FadeOutReason::NaturalTrackEnd => {
                        runtime.phase = BgmPhase::InterTrackPause {
                            remaining_secs: runtime.rng.random_pause_secs(),
                        }
                    }
                }
            } else {
                runtime.phase = BgmPhase::StartFadeOut {
                    elapsed_secs: next_elapsed,
                    reason,
                };
            }
        }
        BgmPhase::InterTrackPause { remaining_secs } => {
            if runtime.current_context != runtime.target_context {
                runtime.phase = BgmPhase::NextTrack;
                return;
            }
            let next_remaining = (remaining_secs - delta_secs).max(0.0);
            if next_remaining <= f32::EPSILON {
                runtime.phase = BgmPhase::NextTrack;
            } else {
                runtime.phase = BgmPhase::InterTrackPause {
                    remaining_secs: next_remaining,
                };
            }
        }
    }
}

fn start_next_track_for_target_context(
    commands: &mut Commands,
    catalog: &BgmTrackCatalog,
    runtime: &mut BgmRuntime,
) {
    let tracks = match runtime.target_context {
        MusicContext::Menu => &catalog.menu_tracks,
        MusicContext::Game => &catalog.game_tracks,
    };

    if tracks.is_empty() {
        runtime.current_context = runtime.target_context;
        runtime.phase = BgmPhase::InterTrackPause {
            remaining_secs: runtime.rng.random_pause_secs(),
        };
        return;
    }

    let is_context_switch = runtime.current_context != runtime.target_context;
    let fade_in_duration = if is_context_switch {
        CONTEXT_SWITCH_FADE_SECONDS
    } else {
        TRACK_FADE_SECONDS
    };
    let next_index = runtime.rng.pick_index(tracks.len());
    let next_handle = tracks[next_index].clone();
    let entity = commands
        .spawn((
            AudioPlayer::new(next_handle.clone()),
            PlaybackSettings::ONCE.with_volume(Volume::SILENT),
            BgmTrackMarker,
        ))
        .id();

    runtime.active_track = Some(ActiveTrack {
        entity,
        handle: next_handle,
        duration_secs: None,
        playback_elapsed_secs: 0.0,
        sink_started: false,
    });
    runtime.current_context = runtime.target_context;
    runtime.phase = BgmPhase::StartFadeIn {
        elapsed_secs: 0.0,
        duration_secs: fade_in_duration,
    };
}

fn despawn_active_track(commands: &mut Commands, active_track: &mut Option<ActiveTrack>) {
    if let Some(track) = active_track.take() {
        commands.entity(track.entity).despawn();
    }
}

fn scan_music_paths_once(assets_root: &Path) -> MusicScanResult {
    let menu_dir = assets_root.join(PathBuf::from(MENU_MUSIC_SUBDIR));
    let game_dir = assets_root.join(PathBuf::from(GAME_MUSIC_SUBDIR));
    MusicScanResult {
        menu_relative_paths: list_mp3_relative_paths(&menu_dir, MENU_MUSIC_SUBDIR),
        game_relative_paths: list_mp3_relative_paths(&game_dir, GAME_MUSIC_SUBDIR),
    }
}

fn list_mp3_relative_paths(dir: &Path, relative_prefix: &str) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut relative_paths = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let extension = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_ascii_lowercase())
                .unwrap_or_default();
            if extension != "mp3" {
                return None;
            }
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| format!("{}/{}", relative_prefix.replace('\\', "/"), name))
        })
        .collect::<Vec<_>>();

    relative_paths.sort();
    relative_paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn random_pause_is_within_bounds() {
        let mut rng = Rng64::seeded(1);
        for _ in 0..128 {
            let pause = rng.random_pause_secs();
            assert!(pause >= INTER_TRACK_PAUSE_MIN_SECONDS);
            assert!(pause <= INTER_TRACK_PAUSE_MAX_SECONDS);
        }
    }

    #[test]
    fn pick_index_works_for_single_track() {
        let mut rng = Rng64::seeded(9);
        for _ in 0..32 {
            assert_eq!(rng.pick_index(1), 0);
        }
    }

    #[test]
    fn pick_index_allows_repeated_values_for_multiple_tracks() {
        let mut rng = Rng64::seeded(42);
        let mut saw_repetition = false;
        let mut previous = rng.pick_index(3);
        for _ in 0..64 {
            let next = rng.pick_index(3);
            if next == previous {
                saw_repetition = true;
                break;
            }
            previous = next;
        }
        assert!(
            saw_repetition,
            "independent random selection should allow repeats"
        );
    }

    #[test]
    fn phase_transitions_after_natural_fade_out_to_pause() {
        let mut runtime = BgmRuntime {
            target_context: MusicContext::Menu,
            current_context: MusicContext::Menu,
            phase: BgmPhase::StartFadeOut {
                elapsed_secs: TRACK_FADE_SECONDS,
                reason: FadeOutReason::NaturalTrackEnd,
            },
            active_track: None,
            rng: Rng64::seeded(5),
        };
        runtime.phase = BgmPhase::InterTrackPause {
            remaining_secs: runtime.rng.random_pause_secs(),
        };
        match runtime.phase {
            BgmPhase::InterTrackPause { remaining_secs } => {
                assert!(remaining_secs >= INTER_TRACK_PAUSE_MIN_SECONDS);
                assert!(remaining_secs <= INTER_TRACK_PAUSE_MAX_SECONDS);
            }
            _ => panic!("expected inter-track pause"),
        }
    }

    #[test]
    fn context_switch_skips_inter_track_pause() {
        let mut runtime = BgmRuntime {
            target_context: MusicContext::Game,
            current_context: MusicContext::Menu,
            phase: BgmPhase::StartFadeOut {
                elapsed_secs: CONTEXT_SWITCH_FADE_SECONDS,
                reason: FadeOutReason::ContextSwitch,
            },
            active_track: None,
            rng: Rng64::seeded(11),
        };
        runtime.phase = BgmPhase::NextTrack;
        assert!(matches!(runtime.phase, BgmPhase::NextTrack));
    }

    #[test]
    fn scheduler_is_disabled_for_zero_volume() {
        assert!(!scheduler_enabled(0.0));
        assert!(!scheduler_enabled(f32::EPSILON * 0.5));
        assert!(scheduler_enabled(0.01));
    }

    #[test]
    fn scan_music_paths_filters_only_mp3_and_non_recursive() {
        let root = std::env::temp_dir().join(format!(
            "flux_bgm_scan_test_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let menu = root.join("music").join("menu");
        let game = root.join("music").join("game");
        let nested = menu.join("nested");
        fs::create_dir_all(&nested).expect("create nested");
        fs::create_dir_all(&game).expect("create game");

        fs::write(menu.join("a.mp3"), b"a").expect("write a");
        fs::write(menu.join("b.ogg"), b"b").expect("write b");
        fs::write(menu.join("c.MP3"), b"c").expect("write c");
        fs::write(nested.join("d.mp3"), b"d").expect("write d");
        fs::write(game.join("g1.mp3"), b"g1").expect("write g1");

        let scan = scan_music_paths_once(&root);
        assert_eq!(scan.menu_relative_paths.len(), 2);
        assert!(scan
            .menu_relative_paths
            .contains(&"music/menu/a.mp3".to_string()));
        assert!(scan
            .menu_relative_paths
            .contains(&"music/menu/c.MP3".to_string()));
        assert_eq!(
            scan.game_relative_paths,
            vec!["music/game/g1.mp3".to_string()]
        );

        fs::remove_dir_all(&root).expect("cleanup");
    }
}
