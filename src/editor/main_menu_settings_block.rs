fn open_settings_screen(menu_ui: &mut MainMenuUiState) {
    menu_ui.screen = MainMenuScreen::Settings;
    menu_ui.settings_tab = MainMenuSettingsTab::Graphics;
    menu_ui.confirm_state = None;
    menu_ui.confirm_text.clear();
    menu_ui.post_save_action = None;
}

fn handle_settings_slider_changes(
    mut changes: EventReader<SliderValueChanged>,
    mut audio_settings: ResMut<AudioSettingsState>,
) {
    for event in changes.read() {
        if event.id != SETTINGS_MUSIC_VOLUME_SLIDER_ID {
            continue;
        }
        audio_settings.set_runtime_music_volume_percent(event.value.max(0) as u32);
    }
}

#[cfg(test)]
mod main_menu_settings_tests {
    use super::open_settings_screen;
    use crate::save::{MainMenuScreen, MainMenuSettingsTab, MainMenuUiState};

    #[test]
    fn opening_settings_selects_settings_screen_and_graphics_tab() {
        let mut ui = MainMenuUiState::default();
        ui.screen = MainMenuScreen::Plugins;
        ui.settings_tab = MainMenuSettingsTab::Sound;
        open_settings_screen(&mut ui);
        assert_eq!(ui.screen, MainMenuScreen::Settings);
        assert_eq!(ui.settings_tab, MainMenuSettingsTab::Graphics);
    }
}
