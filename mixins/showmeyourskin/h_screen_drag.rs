// A held slider in the armour config screen follows the mouse.
fn smys_screen_drag(game: &crate::game::Game<'_>, screen: &horizon_client::mod_api::NativeScreenView<'_>, canceled: &mut bool) {
    crate::game::showmeyourskin::screen_drag(game, screen, canceled);
}
