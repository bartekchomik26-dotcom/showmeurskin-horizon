// Once a frame: queued actions and who is in combat.
fn smys_frame(game: &mut crate::game::Game<'_>) {
    crate::game::showmeyourskin::frame(game);
}
