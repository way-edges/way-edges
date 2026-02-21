use knus::{ast::SpannedNode, errors::DecodeError, traits::ErrorSpan, Decode, DecodeScalar};

pub fn argv<S: ErrorSpan>(
    node: &SpannedNode<S>,
) -> Result<&knus::ast::Value<S>, knus::errors::DecodeError<S>> {
    node.arguments
        .first()
        .ok_or(DecodeError::missing(node, "no argument provided"))
}

pub fn argv_v<S: ErrorSpan, V: DecodeScalar<S>>(
    node: &SpannedNode<S>,
    ctx: &mut knus::decode::Context<S>,
) -> Result<V, knus::errors::DecodeError<S>> {
    let arg = argv(node)?;
    V::decode(arg, ctx)
}

pub fn argv_str<S: ErrorSpan>(
    node: &SpannedNode<S>,
    ctx: &mut knus::decode::Context<S>,
) -> Result<String, knus::errors::DecodeError<S>> {
    let arg = argv(node)?;
    String::decode(arg, ctx)
}

pub fn argv_float<S: ErrorSpan>(
    node: &SpannedNode<S>,
    ctx: &mut knus::decode::Context<S>,
) -> Result<f64, knus::errors::DecodeError<S>> {
    let arg = argv(node)?;
    f64::decode(arg, ctx)
}

pub fn argv_int<S: ErrorSpan>(
    node: &SpannedNode<S>,
    ctx: &mut knus::decode::Context<S>,
) -> Result<i64, knus::errors::DecodeError<S>> {
    let arg = argv(node)?;
    i64::decode(arg, ctx)
}

pub trait ToKdlError<S: ErrorSpan> {
    type Ok;
    fn to_kdl_error(
        self,
        span: &knus::ast::SpannedNode<S>,
    ) -> Result<Self::Ok, knus::errors::DecodeError<S>>;
}

impl<T, E, S> ToKdlError<S> for Result<T, E>
where
    E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    S: knus::traits::ErrorSpan,
{
    type Ok = T;

    fn to_kdl_error(
        self,
        span: &knus::ast::SpannedNode<S>,
    ) -> Result<T, knus::errors::DecodeError<S>> {
        self.map_err(|e| knus::errors::DecodeError::conversion(span, e))
    }
}

#[macro_export]
macro_rules! unexpected_node_name {
    ($name:expr) => {
        log::warn!("unexpected node name '{:?}'", $name);
    };
}
