// A player's hat (a skull, pumpkin or block on the head): hidden when set so.
fn smys_hat(game: &crate::game::Game<'_>, mob: &horizon_client::mobs::Mob) -> ::core::ops::ControlFlow<()> {
    crate::game::showmeyourskin::hat(game, mob)
}
