mod keybindings;
pub use keybindings::{default_helix_insert_keybindings, default_helix_normal_keybindings};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use super::{
    keybindings::{KeyNode, PartialKeySequence},
    EditMode, KeyCombination,
};
use crate::{
    edit_mode::keybindings::Keybindings,
    enums::{EditCommand, HelixEvent, HelixNormal, ReedlineEvent, ReedlineRawEvent},
    PromptEditMode, PromptHelixMode,
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
    partial_key_sequence: Option<PartialKeySequence>,
}

impl Default for Helix {
    fn default() -> Self {
        Self {
            insert_keybindings: Keybindings::default(),
            normal_keybindings: Keybindings::default(),
            mode: Mode::Insert,
            partial_key_sequence: None,
        }
    }
}

fn to_lowercase_key_code(key_code: KeyCode) -> KeyCode {
    if let KeyCode::Char(c) = key_code {
        KeyCode::Char(c.to_ascii_lowercase())
    } else {
        key_code
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

    fn handle_binding(&mut self, modifiers: KeyModifiers, key_code: KeyCode) -> ReedlineEvent {
        let event = if let Some(mut partial_key_sequence) = self.partial_key_sequence.take() {
            if let KeyCode::Char(c) = key_code {
                partial_key_sequence.history.push(c);
            }
            match partial_key_sequence.sequence.map.remove(&KeyCombination {
                modifier: modifiers,
                key_code: to_lowercase_key_code(key_code),
            }) {
                Some(KeyNode::Event(reedline_event)) => reedline_event,
                Some(KeyNode::Sequence(sequence)) => {
                    self.partial_key_sequence = Some(PartialKeySequence {
                        sequence,
                        history: partial_key_sequence.history,
                    });
                    ReedlineEvent::None
                }
                None => match self.mode {
                    Mode::Normal(_) => ReedlineEvent::None,
                    Mode::Insert => ReedlineEvent::Edit(
                        partial_key_sequence
                            .history
                            .into_iter()
                            .map(EditCommand::InsertChar)
                            .collect(),
                    ),
                },
            }
        } else {
            let keybindings = match self.mode {
                Mode::Normal(_) => &self.normal_keybindings,
                Mode::Insert => &self.insert_keybindings,
            };
            match keybindings.find_binding(modifiers, to_lowercase_key_code(key_code)) {
                Some(KeyNode::Sequence(sequence)) => {
                    self.partial_key_sequence = Some(PartialKeySequence {
                        sequence,
                        history: if let KeyCode::Char(c) = key_code {
                            vec![c]
                        } else {
                            vec![]
                        },
                    });
                    ReedlineEvent::None
                }
                Some(KeyNode::Event(reedline_event)) => reedline_event,
                None => match self.mode {
                    Mode::Normal(_) => ReedlineEvent::None,
                    Mode::Insert => {
                        if let KeyCode::Char(c) = key_code {
                            ReedlineEvent::Edit(vec![EditCommand::InsertChar(c)])
                        } else {
                            ReedlineEvent::None
                        }
                    }
                },
            }
        };

        if let ReedlineEvent::Helix(helix_event) = event {
            match helix_event {
                HelixEvent::NormalMode => {
                    let prev_mode = self.mode;
                    self.mode = Mode::Normal(None);
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
                                self.mode = Mode::Insert;
                                ReedlineEvent::Repaint
                            }
                            HelixNormal::SelectMode => {
                                if matches!(self.mode, Mode::Normal(Some(MinorMode::Select))) {
                                    // TODO create extra bindings for minor mode
                                    self.mode = Mode::Normal(None);
                                } else {
                                    self.mode = Mode::Normal(Some(MinorMode::Select));
                                }
                                ReedlineEvent::None
                            }
                            HelixNormal::MoveCharLeft => ReedlineEvent::UntilFound(vec![
                                ReedlineEvent::MenuLeft,
                                ReedlineEvent::Edit(vec![EditCommand::MoveLeft { select }]),
                            ]),
                            HelixNormal::MoveVisualLineDown => ReedlineEvent::UntilFound(vec![
                                ReedlineEvent::MenuDown,
                                ReedlineEvent::Down,
                            ]),
                            HelixNormal::MoveVisualLineUp => ReedlineEvent::UntilFound(vec![
                                ReedlineEvent::MenuUp,
                                ReedlineEvent::Up,
                            ]),
                            HelixNormal::MoveCharRight => ReedlineEvent::UntilFound(vec![
                                ReedlineEvent::HistoryHintComplete,
                                ReedlineEvent::MenuRight,
                                ReedlineEvent::Edit(vec![EditCommand::MoveRight { select }]),
                            ]),
                        }
                    } else {
                        ReedlineEvent::None
                    }
                }
            }
        } else {
            event
        }
    }
}

impl EditMode for Helix {
    fn parse_event(&mut self, event: ReedlineRawEvent) -> ReedlineEvent {
        match event.into() {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => self.handle_binding(modifiers, code),
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
