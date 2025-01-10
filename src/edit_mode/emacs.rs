use std::collections::HashMap;

use crate::{
    edit_mode::{
        keybindings::{
            add_common_control_bindings, add_common_edit_bindings, add_common_navigation_bindings,
            add_common_selection_bindings, edit_bind, KeyCombination, Keybindings,
        },
        EditMode,
    },
    enums::{EditCommand, ReedlineEvent, ReedlineRawEvent},
    LineBuffer, PromptEditMode,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use super::keybindings::{
    to_lowercase_key_code, KeyNode, KeySequenceResult, PartialKeySequence, Sequence,
};

/// Returns the current default emacs keybindings
pub fn default_emacs_keybindings() -> Keybindings {
    use EditCommand as EC;
    use KeyCode as KC;
    use KeyModifiers as KM;

    let mut kb = Keybindings::new();
    add_common_control_bindings(&mut kb);
    add_common_navigation_bindings(&mut kb);
    add_common_edit_bindings(&mut kb);
    add_common_selection_bindings(&mut kb);

    // This could be in common, but in Vi it also changes the mode
    kb.add_binding(
        KeyCombination {
            modifier: KM::NONE,
            key_code: KC::Enter,
        },
        vec![],
        ReedlineEvent::Enter,
    );

    // *** CTRL ***
    // Moves
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('b'),
        },
        vec![],
        ReedlineEvent::UntilFound(vec![ReedlineEvent::MenuLeft, ReedlineEvent::Left]),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('f'),
        },
        vec![],
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::HistoryHintComplete,
            ReedlineEvent::MenuRight,
            ReedlineEvent::Right,
        ]),
    );
    // Undo/Redo
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('g'),
        },
        vec![],
        edit_bind(EC::Redo),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('z'),
        },
        vec![],
        edit_bind(EC::Undo),
    );
    // Cutting
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('y'),
        },
        vec![],
        edit_bind(EC::PasteCutBufferBefore),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('w'),
        },
        vec![],
        edit_bind(EC::CutWordLeft),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('k'),
        },
        vec![],
        edit_bind(EC::CutToLineEnd),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('u'),
        },
        vec![],
        edit_bind(EC::CutFromStart),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Char('d'),
        },
        vec![],
        edit_bind(EC::CutWordRight),
    );
    // Edits
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('t'),
        },
        vec![],
        edit_bind(EC::SwapGraphemes),
    );

    // *** ALT ***
    // Moves
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Left,
        },
        vec![],
        edit_bind(EC::MoveWordLeft { select: false }),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Right,
        },
        vec![],
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::HistoryHintWordComplete,
            edit_bind(EC::MoveWordRight { select: false }),
        ]),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Char('b'),
        },
        vec![],
        edit_bind(EC::MoveWordLeft { select: false }),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Char('f'),
        },
        vec![],
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::HistoryHintWordComplete,
            edit_bind(EC::MoveWordRight { select: false }),
        ]),
    );
    // Edits
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Delete,
        },
        vec![],
        edit_bind(EC::DeleteWord),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Backspace,
        },
        vec![],
        edit_bind(EC::BackspaceWord),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Char('m'),
        },
        vec![],
        ReedlineEvent::Edit(vec![EditCommand::BackspaceWord]),
    );
    // Case changes
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Char('u'),
        },
        vec![],
        edit_bind(EC::UppercaseWord),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Char('l'),
        },
        vec![],
        edit_bind(EC::LowercaseWord),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Char('c'),
        },
        vec![],
        edit_bind(EC::CapitalizeChar),
    );

    kb
}

/// This parses the incoming Events like a emacs style-editor
pub struct Emacs {
    keybindings: Keybindings,
    partial_key_sequence: Option<PartialKeySequence>,
}

impl Default for Emacs {
    fn default() -> Self {
        Emacs {
            keybindings: default_emacs_keybindings(),
            partial_key_sequence: None,
        }
    }
}

impl Emacs {
    /// Emacs style input parsing constructor if you want to use custom keybindings
    pub const fn new(keybindings: Keybindings) -> Self {
        Emacs {
            keybindings,
            partial_key_sequence: None,
        }
    }

    fn cancel_key_sequence(&self, keycombinations: Vec<KeyCombination>) -> ReedlineEvent {
        ReedlineEvent::Multiple(
            keycombinations
                .into_iter()
                .flat_map(|kc| match kc {
                    KeyCombination {
                        modifier: KeyModifiers::SHIFT | KeyModifiers::NONE,
                        key_code: KeyCode::Char(c),
                    } => Some(ReedlineEvent::Edit(vec![EditCommand::InsertChar(c)])),
                    _ => match self
                        .keybindings
                        .find_binding(kc.modifier, to_lowercase_key_code(kc.key_code))?
                    {
                        KeyNode::Event(event) => Some(event),
                        KeyNode::Sequence(_) => unreachable!(""),
                    },
                })
                .collect(),
        )
    }

    fn handle_binding(&mut self, kc: KeyCombination) -> Option<ReedlineEvent> {
        let Some(mut partial_key_sequence) = self.partial_key_sequence.take().or_else(|| {
            self.keybindings
                .find_binding(kc.modifier, to_lowercase_key_code(kc.key_code))
                .map(|key_node| {
                    PartialKeySequence::new(Sequence {
                        map: HashMap::from([(kc.clone(), key_node)]),
                    })
                })
        }) else {
            return if let KeyCode::Char(c) = kc.key_code {
                Some(ReedlineEvent::Edit(vec![EditCommand::InsertChar(c)]))
            } else {
                None
            };
        };

        match partial_key_sequence.advance(kc) {
            KeySequenceResult::Pending => {
                self.partial_key_sequence = Some(partial_key_sequence);
                None
            }
            KeySequenceResult::Matched(reedline_event) => Some(reedline_event),
            KeySequenceResult::Cancelled(keycombinations) => {
                Some(self.cancel_key_sequence(keycombinations))
            }
        }
    }
}

impl EditMode for Emacs {
    fn parse_event(&mut self, _line_buffer: &LineBuffer, event: ReedlineRawEvent) -> ReedlineEvent {
        match event.into() {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => self
                .handle_binding(KeyCombination {
                    modifier: modifiers,
                    key_code: code,
                })
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
        PromptEditMode::Emacs
    }
}

#[cfg(test)]
mod test {
    use crate::edit_mode::keybindings::KeyCombination;

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn ctrl_l_leads_to_clear_screen_event() {
        let mut emacs = Emacs::default();
        let ctrl_l = ReedlineRawEvent::try_from(Event::Key(KeyEvent::new(
            KeyCode::Char('l'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
        let result = emacs.parse_event(ctrl_l);

        assert_eq!(result, ReedlineEvent::ClearScreen);
    }

    #[test]
    fn overriding_default_keybindings_works() {
        let mut keybindings = default_emacs_keybindings();
        keybindings.add_binding(
            KeyCombination {
                modifier: KeyModifiers::CONTROL,
                key_code: KeyCode::Char('l'),
            },
            vec![],
            ReedlineEvent::HistoryHintComplete,
        );

        let mut emacs = Emacs::new(keybindings);
        let ctrl_l = ReedlineRawEvent::try_from(Event::Key(KeyEvent::new(
            KeyCode::Char('l'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
        let result = emacs.parse_event(ctrl_l);

        assert_eq!(result, ReedlineEvent::HistoryHintComplete);
    }

    #[test]
    fn inserting_character_works() {
        let mut emacs = Emacs::default();
        let l = ReedlineRawEvent::try_from(Event::Key(KeyEvent::new(
            KeyCode::Char('l'),
            KeyModifiers::NONE,
        )))
        .unwrap();
        let result = emacs.parse_event(l);

        assert_eq!(
            result,
            ReedlineEvent::Edit(vec![EditCommand::InsertChar('l')])
        );
    }

    #[test]
    fn inserting_capital_character_works() {
        let mut emacs = Emacs::default();

        let uppercase_l = ReedlineRawEvent::try_from(Event::Key(KeyEvent::new(
            KeyCode::Char('l'),
            KeyModifiers::SHIFT,
        )))
        .unwrap();
        let result = emacs.parse_event(uppercase_l);

        assert_eq!(
            result,
            ReedlineEvent::Edit(vec![EditCommand::InsertChar('L')])
        );
    }

    #[test]
    fn return_none_reedline_event_when_keybinding_is_not_found() {
        let keybindings = Keybindings::default();

        let mut emacs = Emacs::new(keybindings);
        let ctrl_l = ReedlineRawEvent::try_from(Event::Key(KeyEvent::new(
            KeyCode::Char('l'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
        let result = emacs.parse_event(ctrl_l);

        assert_eq!(result, ReedlineEvent::None);
    }

    #[test]
    fn inserting_capital_character_for_non_ascii_remains_as_is() {
        let mut emacs = Emacs::default();

        let uppercase_l = ReedlineRawEvent::try_from(Event::Key(KeyEvent::new(
            KeyCode::Char('😀'),
            KeyModifiers::SHIFT,
        )))
        .unwrap();
        let result = emacs.parse_event(uppercase_l);

        assert_eq!(
            result,
            ReedlineEvent::Edit(vec![EditCommand::InsertChar('😀')])
        );
    }
}
