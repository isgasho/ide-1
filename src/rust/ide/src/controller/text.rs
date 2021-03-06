//! Text Controller module.
//!
//! Facade over filesystem API or module text API for text editor. Does discerning between Luna
//! module file and plain text file. In case of luna module idmap and metadata are hidden for the
//! user.

use crate::prelude::*;

use crate::controller::FilePath;

use data::text::TextChange;
use enso_protocol::language_server;
use json_rpc::error::RpcError;
use std::pin::Pin;


// ====================
// === Notification ===
// ====================

/// A notification about changes of file content.
#[derive(Copy,Clone,Debug,Eq,PartialEq)]
pub enum Notification {
    /// The content should be fully reloaded.
    Invalidate,
}



// =======================
// === Text Controller ===
// =======================

/// A handle for file.
///
/// This makes distinction between module and plain text files. The module files are handled by
/// Module Controller, the plain text files are handled directly by File Manager Client.
#[derive(Clone,CloneRef,Debug)]
enum FileHandle {
    PlainText {path:Rc<FilePath>, language_server:Rc<language_server::Connection>},
    Module    {controller:controller::Module },
}

/// A Text Controller Handle.
///
/// This struct contains all information and handles to do all module controller operations.
#[derive(Clone,CloneRef,Debug)]
pub struct Handle {
    file: FileHandle,
}

impl Handle {

    /// Create controller managing plain text file (which is not a module).
    pub fn new_for_plain_text
    (path:FilePath, language_server:Rc<language_server::Connection>) -> Self {
        let path = Rc::new(path);
        Self {
            file : FileHandle::PlainText {path,language_server}
        }
    }

    /// Create controller managing Luna module file.
    pub fn new_for_module(controller:controller::Module) -> Self {
        Self {
            file : FileHandle::Module {controller}
        }
    }

    /// Get clone of file path handled by this controller.
    pub fn file_path(&self) -> &FilePath {
        match &self.file {
            FileHandle::PlainText{path,..} => &*path,
            FileHandle::Module{controller} => controller.path.file_path()
        }
    }

    /// Read file's content.
    pub async fn read_content(&self) -> Result<String,RpcError> {
        use FileHandle::*;
        match &self.file {
            PlainText {path,language_server} => {
                let response = language_server.read_file(&path).await;
                response.map(|response| response.contents)
            },
            Module{controller} => Ok(controller.code())
        }
    }

    /// Store the given content to file.
    pub fn store_content(&self, content:String) -> impl Future<Output=FallibleResult<()>> {
        let file_handle = self.file.clone_ref();
        async move {
            match file_handle {
                FileHandle::PlainText {path,language_server} => {
                    language_server.write_file(&path,&content).await?
                },
                FileHandle::Module {controller} => {
                    controller.check_code_sync(content)?;
                    controller.save_file().await?
                }
            }
            Ok(())
        }
    }

    /// Apply text change.
    ///
    /// This function should be called by view on every user interaction changing the text content
    /// of file. It will e.g. update the Module Controller state and notify other views about
    /// update in case of module files.
    pub fn apply_text_change(&self, change:TextChange) -> FallibleResult<()> {
        if let FileHandle::Module {controller} = &self.file {
            controller.apply_code_change(change)
        } else {
            Ok(())
        }
    }

    /// Get a stream of text changes notifications.
    pub fn subscribe(&self) -> Pin<Box<dyn Stream<Item=Notification>>> {
        match &self.file {
            FileHandle::PlainText{..}       => StreamExt::boxed(futures::stream::empty()),
            FileHandle::Module {controller} => {
                let subscriber = controller.model.subscribe();
                subscriber.filter_map(Self::map_module_notification).boxed()
            }
        }
    }

    async fn map_module_notification
    (notification:model::module::Notification) -> Option<Notification> {
        match notification {
            model::module::Notification::Invalidate      |
            model::module::Notification::CodeChanged{..} => Some(Notification::Invalidate),
            model::module::Notification::MetadataChanged => None,
        }
    }
}



// === Test Utilities ===

#[cfg(test)]
impl Handle {
    /// Get Language Server RPC Client used by this controller.
    pub fn language_server(&self) -> Rc<language_server::Connection> {
        match &self.file {
            FileHandle::PlainText {language_server,..} => language_server.clone_ref(),
            FileHandle::Module {controller}            => controller.language_server.clone_ref()
        }
    }
}



// ============
// === Test ===
// ============

#[cfg(test)]
mod test {
    use super::*;

    use crate::executor::test_utils::TestWithLocalPoolExecutor;

    use data::text::Index;
    use parser::Parser;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn passing_notifications_from_module() {
        let mut test  = TestWithLocalPoolExecutor::set_up();
        test.run_task(async move {
            let ls         = language_server::Connection::new_mock_rc(default());
            let path       = model::module::Path::from_mock_module_name("Test");
            let parser     = Parser::new().unwrap();
            let module_res = controller::Module::new_mock(path,"main = 2+2",default(),ls,parser);
            let module     = module_res.unwrap();
            let controller = Handle::new_for_module(module.clone());
            let mut sub    = controller.subscribe();

            module.apply_code_change(TextChange::insert(Index::new(8),"2".to_string())).unwrap();
            assert_eq!(Some(Notification::Invalidate), sub.next().await);
        })
    }
}
