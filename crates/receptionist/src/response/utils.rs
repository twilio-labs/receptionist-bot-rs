use std::fmt;

use crate::BlockSectionRouter;
use slack_morphism::prelude::*;
use strum::IntoEnumIterator;

pub fn slack_plain_text_input_block_for_view(
    router_variant: BlockSectionRouter,
    index: Option<usize>,
    existing_value: String,
    placeholder: &str,
    label: &str,
) -> Vec<SlackBlock> {
    let input_element =
        SlackBlockPlainTextInputElement::new(router_variant.to_action_id(index), pt!(placeholder));
    let input_element = if existing_value.is_empty() {
        input_element
    } else {
        input_element.with_initial_value(existing_value)
    };

    slack_blocks![some_into(
        SlackInputBlock::new(
            pt!(label),
            SlackInputBlockElement::PlainTextInput(input_element)
        )
        .with_block_id(router_variant.to_block_id(index),)
    )]
}
