use crate::util::{TargetData, path_to_string_with_spaces};
use crate::{AddSystemSerializedParams, AutoPluginContext};
use quote::quote;
use std::cell::RefCell;
use std::collections::HashMap;
use thiserror::Error;

thread_local! {
    static FILE_STATE_MAP: RefCell<HashMap<String, FileState>> = RefCell::new(HashMap::new());
}

// TODO: is there a better way? this originally was using Path instead of String
//  but apparently static references to Path creates "use after free" errors
#[derive(Default)]
pub struct FileState {
    pub plugin_registered: bool,
    pub context: AutoPluginContext,
}

/// Panics if called from outside a procedural macro.
pub fn get_file_path() -> Option<String> {
    use proc_macro2::Span;
    Span::call_site()
        .unwrap()
        .local_file()
        .map(|p| p.display().to_string())
}

pub fn update_file_state<R>(file_path: String, update_fn: impl FnOnce(&mut FileState) -> R) -> R {
    FILE_STATE_MAP.with(|map| {
        let mut map = map.borrow_mut();
        let file_state = map.entry(file_path).or_default();
        update_fn(file_state)
    })
}

pub fn update_state(
    file_path: String,
    target: TargetData,
) -> std::result::Result<(), UpdateStateError> {
    FILE_STATE_MAP.with(|map| {
        let mut map = map.borrow_mut();
        let entry = map.entry(file_path).or_default();
        if entry.plugin_registered {
            return Err(UpdateStateError::PluginAlreadyRegistered);
        }
        let inserted = match target {
            TargetData::RegisterTypes(path) => entry
                .context
                .register_types
                .insert(path_to_string_with_spaces(&path)),
            TargetData::RegisterStateTypes(path) => entry
                .context
                .register_state_types
                .insert(path_to_string_with_spaces(&path)),
            TargetData::AddEvents(path) => entry
                .context
                .add_events
                .insert(path_to_string_with_spaces(&path)),
            TargetData::InitResources(path) => entry
                .context
                .init_resources
                .insert(path_to_string_with_spaces(&path)),
            TargetData::InitStates(path) => entry
                .context
                .init_states
                .insert(path_to_string_with_spaces(&path)),
            TargetData::RequiredComponentAutoName(path) => entry
                .context
                .auto_names
                .insert(path_to_string_with_spaces(&path)),
            TargetData::AddSystem { system, params } => entry
                .context
                .add_systems
                .insert(AddSystemSerializedParams::from_macro_attr(&system, &params)),
        };
        if !inserted {
            return Err(UpdateStateError::Duplicate);
        }
        Ok(())
    })
}

fn get_files_missing_plugin() -> Vec<String> {
    FILE_STATE_MAP.with(|map| {
        let map = map.borrow();
        let mut files_missing_plugin = Vec::new();
        for (file_path, file_state) in map.iter() {
            if file_state.plugin_registered {
                continue;
            }
            files_missing_plugin.push(file_path.clone());
        }
        files_missing_plugin
    })
}

pub fn files_missing_plugin_ts() -> proc_macro2::TokenStream {
    #[allow(unused_mut)]
    let mut output = quote! {};
    let missing_plugin_files = get_files_missing_plugin();
    if !missing_plugin_files.is_empty() {
        #[allow(unused_variables)]
        let messages = missing_plugin_files
            .into_iter()
            .map(|file_path| format!("missing #[auto_plugin(...)] attribute in file: {file_path}"))
            .collect::<Vec<_>>();
        #[cfg(feature = "missing_auto_plugin_is_error")]
        {
            output.extend(messages.iter().map(|message| {
                quote! {
                    log::error!(#message);
                }
            }));
        }
        #[cfg(feature = "missing_auto_plugin_is_warning")]
        {
            output.extend(messages.iter().map(|message| {
                quote! {
                    log::warn!(#message);
                }
            }));
        }
        #[cfg(feature = "missing_auto_plugin_is_compile_error")]
        return syn::Error::new(Span::call_site(), messages.join("\n")).to_compile_error();
    }
    output
}

#[derive(Error, Debug)]
pub enum UpdateStateError {
    #[error("duplicate attribute")]
    Duplicate,
    #[error("plugin already registered above, move plugin fn to the bottom of the file")]
    PluginAlreadyRegistered,
}
