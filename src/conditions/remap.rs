use crate::{
    conditions::{Condition, ConditionConfig, ConditionDescription},
    Event,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct RemapConfig {
    source: String,
}

inventory::submit! {
    ConditionDescription::new::<RemapConfig>("remap")
}

impl_generate_config_from_default!(RemapConfig);

#[typetag::serde(name = "remap")]
impl ConditionConfig for RemapConfig {
    fn build(&self) -> crate::Result<Box<dyn Condition>> {
        // TODO(jean): move this to into a global "immutable functions" array.
        use crate::remap::*;
        let functions: Vec<Box<dyn remap::Function>> = vec![
            Box::new(Split),
            Box::new(ToString),
            Box::new(ToInt),
            Box::new(ToFloat),
            Box::new(ToBool),
            Box::new(ToTimestamp),
            Box::new(Upcase),
            Box::new(Downcase),
            Box::new(UuidV4),
            Box::new(Sha1),
            Box::new(Md5),
            Box::new(Now),
            Box::new(FormatTimestamp),
            Box::new(Contains),
            Box::new(StartsWith),
            Box::new(EndsWith),
            Box::new(Slice),
            Box::new(Tokenize),
            Box::new(Sha2),
            Box::new(Sha3),
            Box::new(ParseDuration),
            Box::new(FormatNumber),
            Box::new(ParseUrl),
            Box::new(Ceil),
            Box::new(Floor),
            Box::new(Round),
            Box::new(ParseSyslog),
            Box::new(ParseTimestamp),
            Box::new(ParseJson),
            Box::new(Truncate),
            Box::new(StripWhitespace),
            Box::new(StripAnsiEscapeCodes),
        ];

        let program = remap::Program::new(&self.source, functions)?;

        Ok(Box::new(Remap { program }))
    }
}

//------------------------------------------------------------------------------

pub struct Remap {
    program: remap::Program,
}

impl Condition for Remap {
    fn check(&self, e: &Event) -> bool {
        // TODO(jean): This clone exists until remap-lang has an "immutable"
        // mode.
        //
        // For now, mutability in reduce "remap ends-when conditions" is
        // allowed, but it won't mutate the original event, since we cloned it
        // here.
        //
        // Having first-class immutability support in the language allows for
        // more performance (one less `clone`), and better compile-time errors
        // when a program tries to mutate the event.
        let mut event = e.clone();

        let mut runtime = remap::Runtime::default();

        runtime
            .execute(&mut event, &self.program)
            .map_err(|_| ())
            .and_then(|opt| match opt {
                Some(value) => match value {
                    remap::Value::Boolean(v) => Ok(v),
                    _ => Err(()),
                },
                None => Err(()),
            })
            .unwrap_or_default()
    }
}
