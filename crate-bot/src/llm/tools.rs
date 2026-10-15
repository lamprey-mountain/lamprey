//! tool definitions and the [`define_tool!`] macro

use serde::Serialize;
use serde_json::Value;
use sdk::http::Http;

use crate::store::Store;

/// additional context for a tool call
#[derive(Clone, Debug)]
pub struct Context {
    pub http: Http,
    pub store: Store,
}

/// a tool the llm can call
pub trait Tool: Send + Sync {
    /// the name the llm uses to call this tool
    fn name(&self) -> &'static str;

    /// metadata sent to the llm describing this tool
    fn definition(&self) -> ToolDef;

    /// execute this tool with the given arguments, returning the result as a string
    fn call(&self, ctx: Context, arguments: Value) -> String;
}

/// a tool definition in the openai tools format
#[derive(Debug, Clone, Serialize)]
pub struct ToolDef {
    #[serde(rename = "type")]
    pub type_: &'static str,
    pub function: FunctionDef,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDef {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: Value,
}

/// a type that can be used as a tool argument, providing its json schema
pub trait ToolArg {
    fn schema() -> Value;
}

macro_rules! impl_tool_arg {
    ($($ty:ty => $json:literal),* $(,)?) => {
        $(
            impl ToolArg for $ty {
                fn schema() -> Value {
                    serde_json::json!({ "type": $json })
                }
            }
        )*
    };
}

impl_tool_arg! {
    String => "string",
    i8 => "integer",
    i16 => "integer",
    i32 => "integer",
    i64 => "integer",
    u8 => "integer",
    u16 => "integer",
    u32 => "integer",
    u64 => "integer",
    f32 => "number",
    f64 => "number",
    bool => "boolean",
}

/// define a tool from a function.
///
/// the doc comment of the function becomes the tool description, every
/// parameter except the [`Context`] becomes a tool argument, and the function
/// must return a `String`.
///
/// the macro expands to a function with the same name that constructs the tool.
///
/// ```ignore
/// define_tool! {
///     /// get the current date/time
///     fn datetime(_ctx: Context) -> String {
///         chrono::Utc::now().to_rfc3339()
///     }
/// }
///
/// let tool = datetime();
/// ```
#[macro_export]
macro_rules! define_tool {
    (
        $(#[doc = $desc:literal])*
        fn $name:ident($ctx:ident: Context $(, $arg:ident: $ty:ty)* $(,)?) -> String
        $body:block
    ) => {
        #[allow(non_snake_case)]
        pub fn $name() -> impl $crate::llm::tools::Tool {
            struct ToolImpl;

            impl $crate::llm::tools::Tool for ToolImpl {
                fn name(&self) -> &'static str {
                    stringify!($name)
                }

                fn definition(&self) -> $crate::llm::tools::ToolDef {
                    $crate::llm::tools::ToolDef {
                        type_: "function",
                        function: $crate::llm::tools::FunctionDef {
                            name: self.name(),
                            description: concat!($($desc, "\n"),*),
                            parameters: ::serde_json::json!({
                                "type": "object",
                                "properties": {
                                    $(
                                        stringify!($arg): <$ty as $crate::llm::tools::ToolArg>::schema(),
                                    )*
                                },
                                "required": [ $(stringify!($arg),)* ],
                            }),
                        },
                    }
                }

                fn call(
                    &self,
                    $ctx: $crate::llm::tools::Context,
                    arguments: ::serde_json::Value,
                ) -> String {
                    let _ = &arguments;
                    $(
                        let $arg: $ty = ::serde_json::from_value(
                            arguments
                                .get(stringify!($arg))
                                .cloned()
                                .unwrap_or(::serde_json::Value::Null),
                        )
                        .expect(concat!("invalid tool argument `", stringify!($arg), "`"));
                    )*
                    $body
                }
            }

            ToolImpl
        }
    };
}

define_tool! {
    /// get the current date/time
    fn datetime(_ctx: Context) -> String {
        chrono::Utc::now().to_rfc3339()
    }
}
