// A player's name tag, hidden when its config says so.
fn smys_name_tag(game: &crate::game::Game<'_>, entity_id: u32, kind: &str, visible: &mut bool) {
    crate::game::showmeyourskin::name_tag(game, entity_id, kind, visible);
}
