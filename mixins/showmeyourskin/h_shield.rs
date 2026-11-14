// A shield resolved for a player's hand: emptied when hidden, without glint when set so.
fn smys_shield(
    value: ItemRenderState,
    game: &crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    display: horizon_assets::item_model::DisplayContext,
    owner: Option<&horizon_assets::item_model::OwnerState>,
) -> ItemRenderState {
    crate::game::showmeyourskin::shield(game, stack, display, owner, value)
}
