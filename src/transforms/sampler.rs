use super::Transform;
use crate::{
    config::{
        log_schema, DataType, GenerateConfig, TransformConfig, TransformContext,
        TransformDescription,
    },
    event::Event,
    internal_events::{SamplerEventDiscarded, SamplerEventProcessed},
};
use regex::RegexSet; // TODO: use regex::bytes
use serde::{Deserialize, Serialize};
use snafu::ResultExt;

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct SamplerConfig {
    pub rate: u64,
    pub key_field: Option<String>,
    #[serde(default)]
    pub pass_list: Vec<String>,
}

inventory::submit! {
    TransformDescription::new::<SamplerConfig>("sampler")
}

impl GenerateConfig for SamplerConfig {
    fn generate_config() -> toml::Value {
        toml::Value::try_from(Self {
            rate: 10,
            key_field: None,
            pass_list: Vec::new(),
        })
        .unwrap()
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "sampler")]
impl TransformConfig for SamplerConfig {
    async fn build(&self, _cx: TransformContext) -> crate::Result<Box<dyn Transform>> {
        Ok(RegexSet::new(&self.pass_list)
            .map::<Box<dyn Transform>, _>(|regex_set| {
                Box::new(Sampler::new(self.rate, self.key_field.clone(), regex_set))
            })
            .context(super::InvalidRegex)?)
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn output_type(&self) -> DataType {
        DataType::Log
    }

    fn transform_type(&self) -> &'static str {
        "sampler"
    }
}

pub struct Sampler {
    rate: u64,
    key_field: String,
    pass_list: RegexSet,
}

impl Sampler {
    pub fn new(rate: u64, key_field: Option<String>, pass_list: RegexSet) -> Self {
        let key_field = key_field.unwrap_or_else(|| log_schema().message_key().to_string());
        Self {
            rate,
            key_field,
            pass_list,
        }
    }
}

impl Transform for Sampler {
    fn transform(&mut self, mut event: Event) -> Option<Event> {
        let message = event
            .as_log()
            .get(&self.key_field)
            .map(|v| v.to_string_lossy())
            .unwrap_or_else(|| "".into());

        emit!(SamplerEventProcessed);

        if self.pass_list.is_match(&message) {
            return Some(event);
        }

        if seahash::hash(message.as_bytes()) % self.rate == 0 {
            event
                .as_mut_log()
                .insert("sample_rate", self.rate.to_string());

            Some(event)
        } else {
            emit!(SamplerEventDiscarded);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;
    use crate::transforms::Transform;
    use approx::assert_relative_eq;
    use regex::RegexSet;

    #[test]
    fn genreate_config() {
        crate::test_util::test_generate_config::<SamplerConfig>();
    }

    #[test]
    fn samples_at_roughly_the_configured_rate() {
        let num_events = 10000;

        let events = random_events(num_events);
        let mut sampler = Sampler::new(2, None, RegexSet::new(&["na"]).unwrap());
        let total_passed = events
            .into_iter()
            .filter_map(|event| sampler.transform(event))
            .count();
        let ideal = 1.0 as f64 / 2.0 as f64;
        let actual = total_passed as f64 / num_events as f64;
        assert_relative_eq!(ideal, actual, epsilon = ideal * 0.5);

        let events = random_events(num_events);
        let mut sampler = Sampler::new(25, None, RegexSet::new(&["na"]).unwrap());
        let total_passed = events
            .into_iter()
            .filter_map(|event| sampler.transform(event))
            .count();
        let ideal = 1.0 as f64 / 25.0 as f64;
        let actual = total_passed as f64 / num_events as f64;
        assert_relative_eq!(ideal, actual, epsilon = ideal * 0.5);
    }

    #[test]
    fn consistently_samples_the_same_events() {
        let events = random_events(1000);
        let mut sampler = Sampler::new(2, None, RegexSet::new(&["na"]).unwrap());

        let first_run = events
            .clone()
            .into_iter()
            .filter_map(|event| sampler.transform(event))
            .collect::<Vec<_>>();
        let second_run = events
            .into_iter()
            .filter_map(|event| sampler.transform(event))
            .collect::<Vec<_>>();

        assert_eq!(first_run, second_run);
    }

    #[test]
    fn always_passes_events_matching_pass_list() {
        let event = Event::from("i am important");
        let mut sampler = Sampler::new(0, None, RegexSet::new(&["important"]).unwrap());
        let iterations = 0..1000;
        let total_passed = iterations
            .filter_map(|_| sampler.transform(event.clone()))
            .count();
        assert_eq!(total_passed, 1000);
    }

    #[test]
    fn handles_key_field() {
        let event = Event::from("nananana");
        let mut sampler = Sampler::new(0, Some("timestamp".into()), RegexSet::new(&[":"]).unwrap());
        let iterations = 0..1000;
        let total_passed = iterations
            .filter_map(|_| sampler.transform(event.clone()))
            .count();
        assert_eq!(total_passed, 1000);
    }

    #[test]
    fn sampler_adds_sampling_rate_to_event() {
        let events = random_events(10000);
        let mut sampler = Sampler::new(10, None, RegexSet::new(&["na"]).unwrap());
        let passing = events
            .into_iter()
            .filter(|s| {
                !s.as_log()[log_schema().message_key()]
                    .to_string_lossy()
                    .contains("na")
            })
            .find_map(|event| sampler.transform(event))
            .unwrap();
        assert_eq!(passing.as_log()["sample_rate"], "10".into());

        let events = random_events(10000);
        let mut sampler = Sampler::new(25, None, RegexSet::new(&["na"]).unwrap());
        let passing = events
            .into_iter()
            .filter(|s| {
                !s.as_log()[log_schema().message_key()]
                    .to_string_lossy()
                    .contains("na")
            })
            .find_map(|event| sampler.transform(event))
            .unwrap();
        assert_eq!(passing.as_log()["sample_rate"], "25".into());

        // If the event passed the regex check, don't include the sampling rate
        let mut sampler = Sampler::new(25, None, RegexSet::new(&["na"]).unwrap());
        let event = Event::from("nananana");
        let passing = sampler.transform(event).unwrap();
        assert!(passing.as_log().get("sample_rate").is_none());
    }

    fn random_events(n: usize) -> Vec<Event> {
        use rand::{thread_rng, Rng};
        use rand_distr::Alphanumeric;

        (0..n)
            .map(|_| {
                thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(10)
                    .collect::<String>()
            })
            .map(Event::from)
            .collect()
    }
}
