use crate::Error;
use serde::{Serialize, de::DeserializeOwned};
use serde_wasm_bindgen as swb;
use wasm_bindgen::{JsCast, prelude::*};

#[wasm_bindgen(module = "/src/js/core.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = "invoke_result")]
    async fn invoke_result_js(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

/// Calls `command` and reads its answer.
///
/// Arguments go over as plain JSON; a number JavaScript cannot hold exactly is
/// refused here rather than rounded on the way.
pub async fn invoke_result<T>(command: &str, args: impl Serialize) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    let args = args
        .serialize(&swb::Serializer::json_compatible())
        .map_err(|why| Error::Serde(why.to_string()))?;
    let answer = invoke_result_js(command, args).await.map_err(refusal)?;
    swb::from_value(answer).map_err(|why| Error::Serde(why.to_string()))
}

fn refusal(why: JsValue) -> Error {
    let said = match (why.as_string(), why.dyn_ref::<js_sys::Error>()) {
        (Some(said), _) => said,
        (None, Some(thrown)) => String::from(thrown.to_string()),
        (None, None) => format!("{why:?}"),
    };
    Error::Command(said)
}
