// The armour config screen.
fn smys_screen_draw(
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
) {
    crate::game::showmeyourskin::screen_draw(game, batch, screen);
}
