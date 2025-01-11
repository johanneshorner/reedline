mod commands;
mod keybindings;
use std::{collections::HashMap, num::NonZeroUsize, sync::Arc};

pub use keybindings::{default_helix_insert_keybindings, default_helix_normal_keybindings};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation;

use super::{
    keybindings::{
        to_lowercase_key_code, KeyNode, KeySequenceResult, PartialKeySequence, Sequence,
    },
    EditMode, KeyCombination,
};
use crate::{
    edit_mode::keybindings::Keybindings,
    enums::{EditCommand, HelixEvent, HelixNormal, ReedlineEvent, ReedlineRawEvent},
    hinter::is_whitespace_str,
    LineBuffer, PromptEditMode, PromptHelixMode,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Mode {
    Normal(Option<MinorMode>),
    Insert,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum MinorMode {
    Select,
    Match,
}

/// This parses incoming input `Event`s like a Helix-Style editor
pub struct Helix {
    insert_keybindings: Keybindings,
    normal_keybindings: Keybindings,
    mode: Mode,
    count: Option<NonZeroUsize>,
    partial_key_sequence: Option<PartialKeySequence>,
    on_next_char: Option<Box<dyn FnOnce(KeyCombination) -> Option<ReedlineEvent> + Send>>,
}

pub struct Asd {
    on_next_char: Option<Box<dyn FnOnce(KeyCombination) -> Option<ReedlineEvent> + Send>>,
}

impl Default for Helix {
    fn default() -> Self {
        Self {
            insert_keybindings: Keybindings::default(),
            normal_keybindings: Keybindings::default(),
            mode: Mode::Insert,
            count: None,
            partial_key_sequence: None,
            on_next_char: None,
        }
    }
}

impl Helix {
    /// Creates Helix editor using defined keybindings
    pub fn new(insert_keybindings: Keybindings, normal_keybindings: Keybindings) -> Self {
        Self {
            insert_keybindings,
            normal_keybindings,
            ..Default::default()
        }
    }

    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.count = None;
        self.partial_key_sequence = None;
        self.on_next_char = None;
    }

    fn active_bindings(&self) -> &Keybindings {
        match self.mode {
            Mode::Normal(_) => &self.normal_keybindings,
            Mode::Insert => &self.insert_keybindings,
        }
    }

    fn cancel_key_sequence(
        &mut self,
        line_buffer: &LineBuffer,
        keycombinations: Vec<KeyCombination>,
    ) -> Option<ReedlineEvent> {
        self.count = None;
        let events: Vec<ReedlineEvent> = keycombinations
            .into_iter()
            .flat_map(|kc| match kc {
                KeyCombination {
                    modifier: KeyModifiers::SHIFT | KeyModifiers::NONE,
                    key_code: KeyCode::Char(c),
                } => matches!(self.mode, Mode::Insert)
                    .then(|| ReedlineEvent::Edit(vec![EditCommand::InsertChar(c)])),
                _ => match self
                    .active_bindings()
                    .find_binding(kc.modifier, to_lowercase_key_code(kc.key_code))?
                {
                    KeyNode::Event(ReedlineEvent::Helix(event)) => {
                        self.handle_helix_event(line_buffer, event)
                    }
                    KeyNode::Event(event) => Some(event),
                    KeyNode::Sequence(_) => unreachable!(""),
                },
            })
            .collect();

        (!events.is_empty()).then_some(ReedlineEvent::Multiple(events))
    }

    fn handle_binding(
        &mut self,
        line_buffer: &LineBuffer,
        kc: KeyCombination,
    ) -> Option<ReedlineEvent> {
        if matches!(kc.key_code, KeyCode::Esc) {
            self.on_next_char = None;
            return if let Some(partial) = self.partial_key_sequence.take() {
                self.cancel_key_sequence(line_buffer, partial.cancel())
            } else {
                self.handle_helix_event(line_buffer, HelixEvent::NormalMode)
            };
        }

        if let Some(on_next_char) = self.on_next_char.take() {
            return on_next_char(kc.clone());
        }

        let Some(mut partial_key_sequence) = self.partial_key_sequence.take().or_else(|| {
            self.active_bindings()
                .find_binding(kc.modifier, to_lowercase_key_code(kc.key_code))
                .map(|key_node| {
                    PartialKeySequence::new(Sequence {
                        map: HashMap::from([(kc.clone(), key_node)]),
                    })
                })
        }) else {
            if let KeyCode::Char(c) = kc.key_code {
                if let Mode::Insert = self.mode {
                    return Some(ReedlineEvent::Edit(vec![EditCommand::InsertChar(c)]));
                }

                match (c, self.count) {
                    ('0'..='9', Some(count)) => {
                        let n = c.to_digit(10).unwrap() as usize;
                        let new_count = count.get() * 10 + n;
                        if count.get() < 100_000_000 {
                            self.count = NonZeroUsize::new(new_count);
                        }
                    }
                    // A non-zero digit will start the count if that number isn't used by a keymap.
                    ('1'..='9', None) => {
                        let n = c.to_digit(10).unwrap() as usize;
                        self.count = NonZeroUsize::new(n);
                    }
                    _ => {}
                }
            }

            return None;
        };
        match partial_key_sequence.advance(kc) {
            KeySequenceResult::Pending => {
                self.partial_key_sequence = Some(partial_key_sequence);
                None
            }
            KeySequenceResult::Matched(ReedlineEvent::Helix(event)) => {
                self.handle_helix_event(line_buffer, event)
            }
            KeySequenceResult::Matched(reedline_event) => Some(reedline_event),
            KeySequenceResult::Cancelled(keycombinations) => {
                self.cancel_key_sequence(line_buffer, keycombinations)
            }
        }
    }

    fn handle_helix_event(
        &mut self,
        line_buffer: &LineBuffer,
        event: HelixEvent,
    ) -> Option<ReedlineEvent> {
        let count = self.count.take().map(|c| c.get()).unwrap_or(1);
        let event = match event {
            HelixEvent::NormalMode => {
                let prev_mode = self.mode;
                self.set_mode(Mode::Normal(None));
                if matches!(prev_mode, Mode::Insert) {
                    ReedlineEvent::Repaint
                } else {
                    ReedlineEvent::None
                }
            }
            HelixEvent::Normal(helix_normal) => {
                if let Mode::Normal(minor_mode) = self.mode {
                    let select = matches!(minor_mode, Some(MinorMode::Select));
                    match helix_normal {
                        HelixNormal::InsertMode => {
                            self.set_mode(Mode::Insert);
                            ReedlineEvent::Repaint
                        }
                        HelixNormal::SelectMode => {
                            if matches!(self.mode, Mode::Normal(Some(MinorMode::Select))) {
                                // TODO create extra bindings for minor mode
                                self.set_mode(Mode::Normal(None));
                            } else {
                                self.set_mode(Mode::Normal(Some(MinorMode::Select)));
                            }
                            ReedlineEvent::None
                        }
                        HelixNormal::MoveCharLeft => apply_multiplier(
                            ReedlineEvent::Edit(vec![EditCommand::MoveLeft { select }]),
                            count,
                        ),
                        HelixNormal::MoveVisualLineDown => {
                            apply_multiplier(ReedlineEvent::Down, count)
                        }
                        HelixNormal::MoveVisualLineUp => apply_multiplier(ReedlineEvent::Up, count),
                        HelixNormal::MoveCharRight => apply_multiplier(
                            ReedlineEvent::Edit(vec![EditCommand::MoveRight { select }]),
                            count,
                        ),
                        HelixNormal::MoveNextWordStart => {
                            let mut base: Vec<EditCommand> =
                                std::iter::repeat(EditCommand::MoveWordRightBeforeStart {
                                    select: true,
                                })
                                .take(count)
                                .collect();
                            if !select {
                                base.insert(base.len() - 1, EditCommand::ClearSelection);
                            }
                            if (is_whitespace_str(grapheme_right_n(line_buffer, 0))
                                && !is_whitespace_str(grapheme_right_n(line_buffer, 1)))
                                || count > 1
                            {
                                base.insert(base.len() - 1, EditCommand::MoveRight { select });
                            }
                            ReedlineEvent::Edit(base)
                        }
                        HelixNormal::MovePrevWordStart => {
                            let mut base: Vec<EditCommand> =
                                std::iter::repeat(EditCommand::MoveWordLeft { select: true })
                                    .take(count)
                                    .collect();
                            if !select {
                                base.insert(base.len() - 1, EditCommand::ClearSelection);
                            }
                            if is_whitespace_str(grapheme_left_n(line_buffer, 1)) {
                                base.insert(base.len() - 1, EditCommand::MoveLeft { select });
                            }
                            ReedlineEvent::Edit(base)
                        }
                        HelixNormal::MoveNextWordEnd => {
                            let mut base: Vec<EditCommand> =
                                std::iter::repeat(EditCommand::MoveWordRightEnd { select: true })
                                    .take(count)
                                    .collect();
                            if !select {
                                base.insert(base.len() - 1, EditCommand::ClearSelection);
                            }
                            if is_whitespace_str(grapheme_right_n(line_buffer, 1)) {
                                base.insert(base.len() - 1, EditCommand::MoveRight { select });
                            }
                            ReedlineEvent::Edit(base)
                        }
                        HelixNormal::MoveNextLongWordStart => {
                            let mut base: Vec<EditCommand> =
                                std::iter::repeat(EditCommand::MoveBigWordRightBeforeStart {
                                    select: true,
                                })
                                .take(count)
                                .collect();
                            if !select {
                                base.insert(base.len() - 1, EditCommand::ClearSelection);
                            }
                            if (is_whitespace_str(grapheme_right_n(line_buffer, 0))
                                && !is_whitespace_str(grapheme_right_n(line_buffer, 1)))
                                || count > 1
                            {
                                base.insert(base.len() - 1, EditCommand::MoveRight { select });
                            }
                            ReedlineEvent::Edit(base)
                        }
                        HelixNormal::MovePrevLongWordStart => {
                            let mut base: Vec<EditCommand> =
                                std::iter::repeat(EditCommand::MoveBigWordLeft { select: true })
                                    .take(count)
                                    .collect();
                            if !select {
                                base.insert(base.len() - 1, EditCommand::ClearSelection);
                            }
                            if is_whitespace_str(grapheme_left_n(line_buffer, 1)) {
                                base.insert(base.len() - 1, EditCommand::MoveLeft { select });
                            }
                            ReedlineEvent::Edit(base)
                        }
                        HelixNormal::MoveNextLongWordEnd => {
                            let mut base: Vec<EditCommand> =
                                std::iter::repeat(EditCommand::MoveBigWordRightEnd {
                                    select: true,
                                })
                                .take(count)
                                .collect();
                            if !select {
                                base.insert(base.len() - 1, EditCommand::ClearSelection);
                            }
                            if is_whitespace_str(grapheme_right_n(line_buffer, 1)) {
                                base.insert(base.len() - 1, EditCommand::MoveRight { select });
                            }
                            ReedlineEvent::Edit(base)
                        }
                        HelixNormal::FindTillChar => {
                            self.on_next_char = Some(Box::new(move |kc: KeyCombination| {
                                if let KeyCode::Char(c) = kc.key_code {
                                    let mut base: Vec<EditCommand> =
                                        std::iter::repeat(EditCommand::MoveRightBefore {
                                            c,
                                            select: true,
                                        })
                                        .take(count)
                                        .collect();
                                    if select {
                                        base.insert(0, EditCommand::Clear);
                                    }
                                    Some(ReedlineEvent::Edit(base))
                                } else {
                                    None
                                }
                            }));
                            ReedlineEvent::None
                        }
                    }
                } else {
                    ReedlineEvent::None
                }
            }
        };

        Some(event)
    }
}

fn grapheme_right_n(line_buffer: &LineBuffer, n: usize) -> &str {
    let buf = &line_buffer.get_buffer()[line_buffer.insertion_point()..];
    buf.graphemes(true).nth(n).unwrap_or(buf)
}

fn grapheme_left_n(line_buffer: &LineBuffer, n: usize) -> &str {
    if line_buffer.insertion_point() > line_buffer.len() - 1 {
        ""
    } else {
        let buf = &line_buffer.get_buffer()[..=line_buffer.insertion_point()];
        buf.graphemes(true).rev().nth(n).unwrap_or(buf)
    }
}

fn apply_multiplier(event: ReedlineEvent, count: usize) -> ReedlineEvent {
    ReedlineEvent::Multiple(std::iter::repeat(event).take(count).collect())
}

impl EditMode for Helix {
    fn parse_event(&mut self, line_buffer: &LineBuffer, event: ReedlineRawEvent) -> ReedlineEvent {
        match event.into() {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => self
                .handle_binding(
                    line_buffer,
                    KeyCombination {
                        modifier: modifiers,
                        key_code: code,
                    },
                )
                .unwrap_or(ReedlineEvent::None),
            Event::Mouse(_) => ReedlineEvent::Mouse,
            Event::Resize(width, height) => ReedlineEvent::Resize(width, height),
            Event::FocusGained => ReedlineEvent::None,
            Event::FocusLost => ReedlineEvent::None,
            Event::Paste(body) => ReedlineEvent::Edit(vec![EditCommand::InsertString(
                body.replace("\r\n", "\n").replace('\r', "\n"),
            )]),
        }
    }

    fn edit_mode(&self) -> PromptEditMode {
        match self.mode {
            Mode::Normal(_) => PromptEditMode::Helix(PromptHelixMode::Normal),
            Mode::Insert => PromptEditMode::Helix(PromptHelixMode::Insert),
        }
    }
}
