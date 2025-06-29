use proc_macro::TokenStream as CompilerStream;
use proc_macro2::TokenStream as MacroStream;

use bevy_auto_plugin_shared::util::{
    ItemWithAttributeMatch, inject_module, items_with_attribute_macro,
};
use bevy_auto_plugin_shared::{generate_add_events, generate_auto_names, generate_init_resources, generate_init_states, generate_register_state_types, generate_register_types, AutoPluginAttribute};
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{Item, ItemMod, Result, parse_macro_input, parse2};


/// Attaches to a module and generates an initialization function that automatically registering types, events, and resources in the `App`.
///
/// # Example
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_auto_plugin::auto_plugin_module::*;
///
/// #[auto_plugin(init_name=init)]
/// pub mod my_plugin {
///     use super::*;
///
///     #[auto_register_type]
///     #[derive(Component)]
///     pub struct MyComponent;
///
///     // code gen:
///     pub(super) fn init(app: &mut App) {  
///         app.register_type::<MyComponent>();
///     }
/// }
///
/// fn plugin(app: &mut App) {
///     app.add_plugin(my_plugin::init)
/// }
/// ```
#[proc_macro_attribute]
pub fn auto_plugin(attr: CompilerStream, input: CompilerStream) -> CompilerStream {
    let mut attrs = bevy_auto_plugin_shared::module::attribute::AutoPluginAttributes::default();
    let arg_parser = syn::meta::parser(|meta| attrs.parse(meta));
    parse_macro_input!(attr with arg_parser);

    // Parse the input module
    let module = parse_macro_input!(input as ItemMod);

    let injected_module = match auto_plugin_inner(module, &attrs.init_name()) {
        Ok(code) => code,
        Err(err) => return err.to_compile_error().into(),
    };

    CompilerStream::from(injected_module)
}

fn auto_plugin_inner(mut module: ItemMod, init_name: &Ident) -> Result<MacroStream> {
    let app_param_ident = Ident::new("app", Span::call_site());
    // Extract the content inside the module
    if let Some((_, items)) = &module.content {
        fn map_to_string(
            iter: impl IntoIterator<Item = ItemWithAttributeMatch>,
        ) -> impl Iterator<Item = String> {
            iter.into_iter()
                .map(ItemWithAttributeMatch::into_path_string)
        }

        // Find all items with the provided [`attribute_name`] #[...] attribute
        let auto_register_types = items_with_attribute_macro(items, AutoPluginAttribute::RegisterType.ident_str())?;
        let auto_register_types = map_to_string(auto_register_types);

        let auto_add_events = items_with_attribute_macro(items, AutoPluginAttribute::AddEvent.ident_str())?;
        let auto_add_events = map_to_string(auto_add_events);

        let auto_init_resources = items_with_attribute_macro(items, AutoPluginAttribute::InitResource.ident_str())?;
        let auto_init_resources = map_to_string(auto_init_resources);

        let auto_names = items_with_attribute_macro(items, AutoPluginAttribute::Name.ident_str())?;
        let auto_names = map_to_string(auto_names);

        let auto_register_state_types =
            items_with_attribute_macro(items, AutoPluginAttribute::RegisterStateType.ident_str())?;
        let auto_register_state_types = map_to_string(auto_register_state_types);

        let auto_init_states = items_with_attribute_macro(items, AutoPluginAttribute::InitState.ident_str())?;
        let auto_init_states = map_to_string(auto_init_states);

        inject_module(&mut module, move || {
            let auto_register_types =
                generate_register_types(&app_param_ident, auto_register_types)?;
            let auto_add_events = generate_add_events(&app_param_ident, auto_add_events)?;
            let auto_init_resources =
                generate_init_resources(&app_param_ident, auto_init_resources)?;
            let auto_names = generate_auto_names(&app_param_ident, auto_names)?;
            let auto_register_state_types =
                generate_register_state_types(&app_param_ident, auto_register_state_types)?;
            let auto_init_states = generate_init_states(&app_param_ident, auto_init_states)?;
            parse2::<Item>(quote! {
                pub(super) fn #init_name(app: &mut bevy_app::prelude::App) {
                    #auto_register_types
                    #auto_register_state_types
                    #auto_add_events
                    #auto_init_resources
                    #auto_init_states
                    #auto_names
                }
            })
        })?;
    }

    let output = quote! {
        #module
    };

    Ok(output)
}

/// Automatically registers a type with the Bevy `App`.
///
/// # Example (without generics)
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_auto_plugin::auto_plugin_module::*;
///
/// #[auto_plugin(init_name=init)]
/// pub mod my_plugin {
///     use super::*;
///
///     #[auto_register_type]
///     #[derive(Component, Reflect)]
///     #[reflect(Component)]
///     struct FooComponent;
///
///     // code gen:
///     pub(super) fn init(app: &mut App) {  
///         app.register_type::<FooComponent>();
///     }
/// }
///
/// fn plugin(app: &mut App) {
///     app.add_plugin(my_plugin::init)
/// }
/// ```
///
/// # Example (with generics)
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_auto_plugin::auto_plugin::*;
///
/// #[auto_plugin(init_name=init)]
/// pub mod my_plugin {
///     use super::*;
///
///     #[auto_register_type(FooComponentWithGeneric<bool>)]
///     #[auto_register_type(FooComponentWithGeneric<u32>)]
///     #[derive(Component, Reflect)]
///     #[reflect(Component)]
///     struct FooComponentWithGeneric<T>(T);
///
///     // code gen:
///     pub(super) fn init(app: &mut App) {  
///         app.register_type::<FooComponentWithGeneric<bool>>();
///         app.register_type::<FooComponentWithGeneric<u32>>();
///     }
/// }
///
/// fn plugin(app: &mut App) {
///     app.add_plugin(my_plugin::init)
/// }
/// ```
#[proc_macro_attribute]
pub fn auto_register_type(_args: CompilerStream, input: CompilerStream) -> CompilerStream {
    // Just return the input unchanged; this acts as a marker.
    input
}
/// Automatically adds an event type to the Bevy `App`.
///
/// # Example (without generics)
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_auto_plugin::auto_plugin::*;
///
/// #[auto_plugin(init_name=init)]
/// pub mod my_plugin {
///     use super::*;
///
///     #[auto_add_event]
///     #[derive(Event, Reflect)]
///     struct FooEvent;
///
///     // code gen:
///     pub(super) fn init(app: &mut App) {  
///         app.add_event::<FooEvent>();
///     }
/// }
///
/// fn plugin(app: &mut App) {
///     app.add_plugin(my_plugin::init)
/// }
/// ```
///
/// # Example (with generics)
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_auto_plugin::auto_plugin::*;
///
/// #[auto_plugin(init_name=init)]
/// pub mod my_plugin {
///     use super::*;
///
///     #[auto_add_event(FooEventWithGeneric<bool>)]
///     #[derive(Event, Reflect)]
///     struct FooEventWithGeneric<T>(T);
///
///     // code gen:
///     pub(super) fn init(app: &mut App) {
///         app.add_event::<FooEventWithGeneric<bool>>();
///     }
/// }
///
/// fn plugin(app: &mut App) {
///     app.add_plugin(my_plugin::init)
/// }
/// ```
#[proc_macro_attribute]
pub fn auto_add_event(_args: CompilerStream, input: CompilerStream) -> CompilerStream {
    // Just return the input unchanged; this acts as a marker.
    input
}
/// Automatically initializes a resource in the Bevy `App`.
///
/// # Example (without generics)
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_auto_plugin::auto_plugin::*;
///
/// #[auto_plugin(init_name=init)]
/// pub mod my_plugin {
///     use super::*;
///
///     #[auto_init_resource]
///     #[derive(Resource, Default, Reflect)]
///     #[reflect(Resource)]
///     struct FooResource;
///
///     // code gen:
///     pub(super) fn init(app: &mut App) {  
///         app.init_resource::<FooResource>();
///     }
/// }
///
/// fn plugin(app: &mut App) {
///     app.add_plugin(my_plugin::init)
/// }
/// ```
/// # Example (with generics)
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_auto_plugin::auto_plugin::*;
///
/// #[auto_plugin(init_name=init)]
/// pub mod my_plugin {
///     use super::*;
///
///     #[auto_init_resource(FooResourceWithGeneric<bool>)]
///     #[derive(Resource, Default, Reflect)]
///     #[reflect(Resource)]
///     struct FooResourceWithGeneric<T>(T);
///
///     // code gen:
///     pub(super) fn init(app: &mut App) {  
///         app.init_resource::<FooResourceWithGeneric<bool>>();
///     }
/// }
///
/// fn plugin(app: &mut App) {
///     app.add_plugin(my_plugin::init)
/// }
/// ```
#[proc_macro_attribute]
pub fn auto_init_resource(_args: CompilerStream, input: CompilerStream) -> CompilerStream {
    // Just return the input unchanged; this acts as a marker.
    input
}
/// Automatically associates a required component `Name` with the default value set to the ident in the Bevy `App`.
/// # Example (without generics)
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_auto_plugin::auto_plugin::*;
///
/// #[auto_plugin(init_name=init)]
/// pub mod my_plugin {
///     use super::*;
///
///     #[auto_register_type]
///     #[derive(Component, Reflect)]
///     #[reflect(Component)]
///     #[auto_name]
///     struct FooComponent;
///
///     // code gen:
///     pub(super) fn init(app: &mut App) {
///         app.register_type::<FooComponent>();
///         app.register_required_components_with::<FooComponent, Name>(|| Name::new("FooComponent"));
///     }
/// }
///
/// fn plugin(app: &mut App) {
///     app.add_plugin(my_plugin::init)
/// }
/// ```
///
/// # Example (with generics)
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_auto_plugin::auto_plugin::*;
///
/// #[auto_plugin(init_name=init)]
/// pub mod my_plugin {
///     use super::*;
///
///     #[auto_register_type(FooComponentWithGeneric<bool>)]
///     #[auto_register_type(FooComponentWithGeneric<u32>)]
///     #[derive(Component, Reflect)]
///     #[reflect(Component)]
///     #[auto_name(FooComponentWithGeneric<bool>)]
///     struct FooComponentWithGeneric<T>(T);
///
///     // code gen:
///     pub(super) fn init(app: &mut App) {  
///         app.register_type::<FooComponentWithGeneric<bool>>();
///         app.register_type::<FooComponentWithGeneric<u32>>();
///         app.register_required_components_with::<FooComponentWithGeneric<boo>, Name>(|| Name::new("FooComponentWithGeneric<boo>"));
///     }
/// }
///
/// fn plugin(app: &mut App) {
///     app.add_plugin(my_plugin::init)
/// }
/// ```
#[proc_macro_attribute]
pub fn auto_name(_attr: CompilerStream, input: CompilerStream) -> CompilerStream {
    // Just return the input unchanged; this acts as a marker.
    input
}

/// Automatically initializes a State in the Bevy `App`.
///
/// # Example (without generics)
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_auto_plugin::auto_plugin::*;
///
/// #[auto_plugin(init_name=init)]
/// pub mod my_plugin {
///     use super::*;
///
///     #[auto_init_state]
///     #[derive(States, Debug, Copy, Clone, Default, PartialEq, Eq, Hash)]
///     enum Foo {
///         #[default]
///         A,
///     }
///
///     // code gen:
///     pub(super) fn init(app: &mut App) {  
///         app.init_state::<FooResource>();
///     }
/// }
///
/// fn plugin(app: &mut App) {
///     app.add_plugin(my_plugin::init)
/// }
/// ```
#[proc_macro_attribute]
pub fn auto_init_state(_attr: CompilerStream, input: CompilerStream) -> CompilerStream {
    // Just return the input unchanged; this acts as a marker.
    input
}

/// Automatically registers a State<T> and NextState<T> in the Bevy `App`.
///
/// # Example (without generics)
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_auto_plugin::auto_plugin::*;
///
/// #[auto_plugin(init_name=init)]
/// pub mod my_plugin {
///     use super::*;
///
///     #[auto_register_state_type]
///     #[derive(States, Debug, Copy, Clone, Default, PartialEq, Eq, Hash)]
///     enum Foo {
///         #[default]
///         A,
///     }
///
///     // code gen:
///     pub(super) fn init(app: &mut App) {  
///         app.register_type::<State<Foo>>();
///         app.register_type::<NextState<Foo>>();
///     }
/// }
///
/// fn plugin(app: &mut App) {
///     app.add_plugin(my_plugin::init)
/// }
/// ```
#[proc_macro_attribute]
pub fn auto_register_state_type(_attr: CompilerStream, input: CompilerStream) -> CompilerStream {
    // Just return the input unchanged; this acts as a marker.
    input
}
