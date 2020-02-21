//! This module contains implementation of ViewLayout with a single TextEditor temporarily
//! occupying half bottom of the screen as the default layout.

use crate::prelude::*;

use crate::controller::text::Handle;
use crate::view::KeyboardListener;
use crate::view::temporary_panel::TemporaryPadding;
use crate::view::temporary_panel::TemporaryPanel;
use crate::view::text_editor::TextEditor;

use basegl::display::world::World;
use nalgebra::zero;
use nalgebra::Vector2;
use std::rc::Rc;
use std::cell::RefCell;
use web_sys::KeyboardEvent;



// ==================
// === LayoutMode ===
// ==================
//TODO: LayoutMode is a temporary enumeration, it will be replaced by proper Panel impl.

/// Defines the element's layout mode. It can fully occupy the screen or only half of it.
#[derive(Clone,Copy,Debug)]
pub enum LayoutMode {
    #[allow(missing_docs)]
    Full,
    #[allow(missing_docs)]
    Half
}

impl Default for LayoutMode {
    fn default() -> Self {
        Self::Half
    }
}



// ==================
// === ViewLayout ===
// ==================

shared! { ViewLayout

/// Initial implementation of ViewLayout with a single TextEditor. Pressing ctrl+f toggles
/// fullscreen mode.
#[derive(Debug)]
pub struct ViewLayoutData {
    text_editor  : TextEditor,
    key_listener : Option<KeyboardListener>,
    layout_mode  : LayoutMode,
    size         : Vector2<f32>
}

impl {
    /// Switches LayoutMode between Half and Full.
    pub fn switch_layout_mode(&mut self) {
        if let LayoutMode::Half = self.layout_mode {
            self.set_layout_mode(LayoutMode::Full)
        } else {
            self.set_layout_mode(LayoutMode::Half)
        }
    }

    /// Sets ViewLayout size.
    pub fn set_size(&mut self, size:Vector2<f32>) {
        self.size = size;
        self.recalculate_layout();
    }
}}


// === Private Methods ===

impl ViewLayoutData {
    fn set_layout_mode(&mut self, layout_mode:LayoutMode) {
        self.layout_mode = layout_mode;
        self.recalculate_layout();
    }

    fn recalculate_layout(&mut self) {
        let size            = self.size;
        let (position,size) = match self.layout_mode {
            LayoutMode::Full => {
                let position   = Vector2::new(0.0,size.y);
                (position,size)
            },
            LayoutMode::Half => {
                let position = Vector2::new(0.0,size.y / 2.0);
                let size     = Vector2::new(size.x,size.y / 2.0);
                (position,size)
            }
        };
        let padding = TemporaryPadding {
            left   : 10.0,
            top    : 0.0,
            right  : 10.0,
            bottom : 0.0
        };
        self.text_editor.set_padding(padding);
        self.text_editor.set_size(size);
        self.text_editor.set_position(position);
        self.text_editor.update();
    }
}

impl ViewLayout {
    /// Creates a new ViewLayout with a single TextEditor.
    pub fn new(world:&World, controller:Handle) -> Self {
        let text_editor  = TextEditor::new(&world,controller);
        let key_listener = None;
        let layout_mode  = default();
        let size         = zero();
        let data         = ViewLayoutData {text_editor,key_listener,layout_mode,size};
        let rc           = Rc::new(RefCell::new(data));
        Self {rc}.init(world)
    }

    fn init_keyboard(self) -> Self {
        let view_layout = self.clone();
        let closure     = move |event:KeyboardEvent| {
            const F_KEY : u32 = 70;
            if event.ctrl_key() && event.key_code() == F_KEY {
                view_layout.switch_layout_mode();
                event.prevent_default();
            }
        };
        let closure : Box<dyn FnMut(KeyboardEvent)> = Box::new(closure);
        let callback                                = Closure::wrap(closure);
        let key_listener = KeyboardListener::new("keydown".into(), callback);
        self.rc.borrow_mut().key_listener = Some(key_listener);
        self
    }

    fn init(self, world:&World) -> Self {
        let screen = world.scene().camera().screen();
        let size   = Vector2::new(screen.width,screen.height);
        self.set_size(size);
        self.init_keyboard()
    }
}
