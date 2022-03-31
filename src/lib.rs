//! The [Neon][neon] crate provides bindings for writing [Node.js addons][addons]
//! (i.e., dynamically-loaded binary modules) with a safe and fast Rust API.
//!
//! ## Getting Started
//!
//! You can conveniently bootstrap a new Neon project with the Neon project
//! generator. You don't need to install anything special on your machine as
//! long as you have a [supported version of Node and Rust][supported] on
//! your system.
//!
//! To start a new project, open a terminal in the directory where you would
//! like to place the project, and run at the command prompt:
//!
//! ```text
//! % npm init neon my-project
//! ... answer the user prompts ...
//! ✨ Created Neon project `my-project`. Happy 🦀 hacking! ✨
//! ```
//!
//! where `my-project` can be any name you like for the project. This will
//! run the Neon project generator, prompting you with a few questions and
//! placing a simple but working Neon project in a subdirectory called
//! `my-project` (or whatever name you chose).
//!
//! You can then install and build the project by changing into the project
//! directory and running the standard Node installation command:
//!
//! ```text
//! % cd my-project
//! % npm install
//! % node
//! > require(".").hello()
//! 'hello node'
//! ```
//!
//! You can look in the project's generated `README.md` for more details on
//! the project structure.
//!
//! ## Example
//!
//! The generated `src/lib.rs` contains a function annotated with the
//! [`#[neon::main]`](main) attribute, marking it as the module's main entry
//! point to be executed when the module is loaded. This function can have
//! any name but is conventionally called `main`:
//!
//! ```no_run
//! # use neon::prelude::*;
//! #
//! # fn hello(mut cx: FunctionContext) -> JsResult<JsString> {
//! #    Ok(cx.string("hello node"))
//! # }
//! #
//! #[neon::main]
//! fn lib(mut cx: ModuleContext) -> NeonResult<()> {
//!     cx.export_function("hello", hello)?;
//!     Ok(())
//! }
//! ```
//!
//! The example code generated by `npm init neon` exports a single
//! function via [`ModuleContext::export_function`](context::ModuleContext::export_function).
//! The `hello` function is defined just above `main` in `src/lib.rs`:
//!
//! ```
//! # use neon::prelude::*;
//! #
//! fn hello(mut cx: FunctionContext) -> JsResult<JsString> {
//!     Ok(cx.string("hello node"))
//! }
//! ```
//!
//! The `hello` function takes a [`FunctionContext`](context::FunctionContext) and
//! returns a JavaScript string. Because all Neon functions can potentially throw a
//! JavaScript exception, the return type is wrapped in a [`JsResult`](result::JsResult).
//!
//! [neon]: https://www.neon-bindings.com/
//! [addons]: https://nodejs.org/api/addons.html
//! [supported]: https://github.com/neon-bindings/neon#platform-support
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod context;
pub mod event;
pub mod handle;
pub mod meta;
pub mod object;
pub mod prelude;
pub mod reflect;
pub mod result;
pub mod types;

#[doc(hidden)]
pub mod macro_internal;

pub use neon_macros::*;

#[cfg(feature = "napi-6")]
mod lifecycle;
