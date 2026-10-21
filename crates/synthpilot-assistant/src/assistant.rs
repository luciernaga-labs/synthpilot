use std::collections::BTreeMap;
use std::fmt::Write as _;

use thiserror::Error;

use super::config::Config;
use super::plan::Proposal;
use super::provider::{self, Role};

pub trait Transport: Send + Sync {
    fn post(
        &self,
        url: &str,
        headers: &[(&'static str, String)],
        body: &str,
    ) -> Result<(u16, String), String>;
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssistantError {
    #[error("Couldn't reach the AI service: {0}")]
    Network(String),
    #[error("The AI service answered with an error ({status}): {message}")]
    Service { status: u16, message: String },
    #[error("The AI's answer couldn't be read: {0}")]
    Format(String),
}

const SYSTEM: &str = "You are a sound designer working on a hardware synthesizer. \
You change the sound currently loaded on the keyboard by setting parameters from the catalog the user sends. \
Rules: \
1. Use only parameter ids from the catalog, exactly as written. \
2. Every value is an integer inside the parameter's range. For options, use the number before \"=\". \
3. Every change names its Part with \"part\" (the Part number); Performance effects take \"part\": null. Parameters marked \"per element\", \"per operator\" or \"per key\" need \"element\"; all others need \"element\": null. \
4. The sound is a Performance of several Parts that play together. The Part open in the editor is shown in full and the other Parts are summarized. Work on every Part the request concerns, not only the open one: balance their Volume, Pan and effect sends, layer them with Velocity Limit or split the keyboard with Note Limit, and look up \"Part <number>\" before changing another Part's elements or operators. When a Part needs a different instrument (a pad, bells, strings) or the request adds an instrument, load a preset sound into a Part with \"load\": an empty Part to add it, or the Part to replace. Look up \"Sounds: <group>\" first to pick the number. A request about one instrument usually concerns the Part that plays it; a request about the whole sound (the mix, space, a layer or a split) concerns several Parts. \
5. Start from the current values and change only what serves the request. Keep the list short: at most 60 changes. \
6. Some parameters, such as the Wave, take a number from a long list shown only as groups. To choose one, first answer only {\"lookup\": [\"Group/Name\", ...]} with up to 8 groups; you will get the numbered options. You can look things up a few times before the final JSON. Pick the Wave first when the request is about a kind of instrument, because it shapes the sound most. Effect types work the same way: ask for \"Variation Type: SPX CHORUS\" to see that type's parameters, then set the type and its parameters in the same answer. \
7. Shape the sound with the envelopes, filter, filter envelope, velocity sensitivity, LFO, EQ and effects the catalog offers. If the request needs something that is not in the catalog, say so in the summary. \
8. Sound design guidance: Elements that play at the same time add up, so when you turn on more than one, lower their Element Level and give them different Velocity Limit ranges (with a little Velocity Cross Fade) instead of stacking them over the whole range; leave at most two Elements sounding at any velocity. Waves named Soft, Med and Hard are the same instrument recorded at different strengths, so map them to low, middle and high velocities; waves named Sw already switch by velocity on their own. A percussive hammer or tine attack comes from the hard layer on high velocities, AEG Attack Time 0, a positive Filter Cutoff Velocity Sensitivity, a fast FEG attack with a short FEG Decay 1, and a positive AEG Time Velocity Sensitivity. Keep the Part Volume and the sum of the levels sensible rather than raising everything. \
9. Write the summary and every reason in English, in one short sentence each, whatever language the request is in. \
Answer with JSON only, in this shape: \
{\"summary\": \"...\", \"changes\": [{\"parameter\": \"id\", \"part\": 1, \"element\": null, \"value\": 0, \"reason\": \"...\"}], \"load\": [{\"part\": 2, \"sound\": 955, \"reason\": \"...\"}]}. Leave \"load\" empty when no sound needs loading.";

pub struct Assistant {
    config: Config,
    transport: Box<dyn Transport>,
}

impl Assistant {
    pub fn new(config: Config, transport: Box<dyn Transport>) -> Self {
        Self { config, transport }
    }

    pub const fn config(&self) -> &Config {
        &self.config
    }

    pub fn propose(
        &self,
        catalog: &str,
        listings: &BTreeMap<String, String>,
        request: &str,
    ) -> Result<Proposal, AssistantError> {
        let mut turns = vec![(Role::User, format!("{catalog}\n## Request\n{request}"))];
        let mut round = 0;
        loop {
            let text = self.ask(&turns)?;
            let proposal = Proposal::parse(&text).map_err(AssistantError::Format)?;
            if proposal.lookup.is_empty()
                || !proposal.changes.is_empty()
                || !proposal.load.is_empty()
                || round == MAX_LOOKUPS
            {
                return Ok(proposal);
            }
            round += 1;
            turns.push((Role::Assistant, text));
            turns.push((
                Role::User,
                options(listings, &proposal.lookup, round == MAX_LOOKUPS),
            ));
        }
    }

    fn ask(&self, turns: &[(Role, String)]) -> Result<String, AssistantError> {
        let request = provider::request(&self.config, SYSTEM, turns);
        let (status, body) = self
            .transport
            .post(&request.url, &request.headers, &request.body)
            .map_err(AssistantError::Network)?;
        provider::reply(self.config.provider, status, &body)
    }
}

const MAX_GROUPS: usize = 8;
/// How many times the model may look things up before it must answer. Three
/// rounds let it open another Part, then pick that Part's Wave and effect.
const MAX_LOOKUPS: usize = 3;

fn options(listings: &BTreeMap<String, String>, groups: &[String], final_round: bool) -> String {
    let mut text = String::from("## Options\n");
    for group in groups.iter().take(MAX_GROUPS) {
        let found = listings
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(group.trim()));
        match found {
            Some((name, list)) => {
                let _ = writeln!(text, "### {name}\n{list}");
            }
            None => {
                let _ = writeln!(text, "### {group}\nNo such group.");
            }
        }
    }
    text.push_str(if final_round {
        "\nNow answer with the final JSON, using the numbers above. Do not ask for more groups."
    } else {
        "\nAnswer with the final JSON, using the numbers above, or look up more if you still need to."
    });
    text
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::config::Provider;

    type Calls = Arc<Mutex<Vec<(String, String)>>>;

    struct Fake {
        reply: Result<(u16, String), String>,
        calls: Calls,
    }

    impl Transport for Fake {
        fn post(
            &self,
            url: &str,
            _headers: &[(&'static str, String)],
            body: &str,
        ) -> Result<(u16, String), String> {
            self.calls
                .lock()
                .unwrap()
                .push((url.to_owned(), body.to_owned()));
            self.reply.clone()
        }
    }

    fn assistant(reply: Result<(u16, String), String>) -> (Assistant, Calls) {
        let calls = Calls::default();
        let config = Config {
            provider: Provider::OpenAi,
            model: "m".to_owned(),
            api_key: "k".to_owned(),
            base_url: "http://local".to_owned(),
        };
        let fake = Fake {
            reply,
            calls: calls.clone(),
        };
        (Assistant::new(config, Box::new(fake)), calls)
    }

    #[test]
    fn sends_the_catalog_and_reads_the_proposal() {
        let content = r#"{\"summary\":\"Crystal\",\"changes\":[{\"parameter\":\"part.volume\",\"value\":90,\"reason\":\"r\"}]}"#;
        let body = format!(r#"{{"choices":[{{"message":{{"content":"{content}"}}}}]}}"#);
        let (assistant, calls) = assistant(Ok((200, body)));
        assert_eq!(assistant.config().model, "m");
        let proposal = assistant
            .propose("CATALOG", &BTreeMap::new(), "crystal rhodes")
            .unwrap();
        assert_eq!(proposal.summary, "Crystal");
        assert_eq!(proposal.changes.len(), 1);
        let calls = calls.lock().unwrap();
        assert_eq!(calls[0].0, "http://local/chat/completions");
        assert!(calls[0].1.contains("CATALOG\\n## Request\\ncrystal rhodes"));
        assert!(calls[0].1.contains("sound designer"));
        assert!(calls[0].1.contains("every reason in English"));
    }

    #[test]
    fn reports_each_kind_of_failure() {
        let (offline, _) = assistant(Err("dns error".to_owned()));
        assert_eq!(
            offline.propose("", &BTreeMap::new(), "x"),
            Err(AssistantError::Network("dns error".to_owned()))
        );
        assert_eq!(
            AssistantError::Network("dns error".to_owned()).to_string(),
            "Couldn't reach the AI service: dns error"
        );
        let body = r#"{"choices":[{"message":{"content":"sorry, no"}}]}"#.to_owned();
        let (chatty, _) = assistant(Ok((200, body)));
        assert!(matches!(
            chatty.propose("", &BTreeMap::new(), "x"),
            Err(AssistantError::Format(_))
        ));
        let (denied, _) = assistant(Ok((429, "{\"error\":{\"message\":\"quota\"}}".to_owned())));
        assert_eq!(
            denied
                .propose("", &BTreeMap::new(), "x")
                .unwrap_err()
                .to_string(),
            "The AI service answered with an error (429): quota"
        );
    }

    struct Script {
        replies: Mutex<Vec<String>>,
        calls: Calls,
    }

    impl Transport for Script {
        fn post(
            &self,
            url: &str,
            _headers: &[(&'static str, String)],
            body: &str,
        ) -> Result<(u16, String), String> {
            self.calls
                .lock()
                .unwrap()
                .push((url.to_owned(), body.to_owned()));
            let content = self.replies.lock().unwrap().remove(0);
            let escaped = content.replace('"', "\\\"");
            Ok((
                200,
                format!(r#"{{"choices":[{{"message":{{"content":"{escaped}"}}}}]}}"#),
            ))
        }
    }

    #[test]
    fn looks_up_groups_before_answering() {
        let calls = Calls::default();
        let script = Script {
            replies: Mutex::new(vec![
                r#"{"lookup":["keys/rd","Made/Up"]}"#.to_owned(),
                r#"{"summary":"Rhodes","changes":[{"parameter":"awm2.wave-number","element":1,"value":166}]}"#.to_owned(),
            ]),
            calls: calls.clone(),
        };
        let config = Config {
            provider: Provider::OpenAi,
            model: "m".to_owned(),
            api_key: "k".to_owned(),
            base_url: "http://local".to_owned(),
        };
        let assistant = Assistant::new(config, Box::new(script));
        let listings = BTreeMap::from([("Keys/Rd".to_owned(), "166=EP1 Sw".to_owned())]);
        let proposal = assistant.propose("CATALOG", &listings, "rhodes").unwrap();
        assert_eq!(proposal.summary, "Rhodes");
        assert_eq!(proposal.changes.len(), 1);
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert!(calls[1].1.contains("### Keys/Rd\\n166=EP1 Sw"));
        assert!(calls[1].1.contains("### Made/Up\\nNo such group."));
        assert!(calls[1].1.contains("\"role\":\"assistant\""));
    }

    #[test]
    fn can_look_up_another_part_and_then_its_waves() {
        let calls = Calls::default();
        let script = Script {
            replies: Mutex::new(vec![
                r#"{"lookup":["Part 6"]}"#.to_owned(),
                r#"{"lookup":["Pad/Warm"]}"#.to_owned(),
                r#"{"lookup":["Pad/Warm"]}"#.to_owned(),
                r#"{"lookup":["Pad/Warm"]}"#.to_owned(),
            ]),
            calls: calls.clone(),
        };
        let config = Config {
            provider: Provider::OpenAi,
            model: "m".to_owned(),
            api_key: "k".to_owned(),
            base_url: "http://local".to_owned(),
        };
        let assistant = Assistant::new(config, Box::new(script));
        let listings = BTreeMap::from([
            ("Part 6".to_owned(), "## Part 6 · AWM2".to_owned()),
            ("Pad/Warm".to_owned(), "1=Warm".to_owned()),
        ]);
        let proposal = assistant.propose("CATALOG", &listings, "pad").unwrap();
        assert!(proposal.changes.is_empty());
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 1 + MAX_LOOKUPS);
        assert!(calls[1].1.contains("### Part 6\\n## Part 6 · AWM2"));
        assert!(calls[1].1.contains("or look up more"));
        assert!(calls[MAX_LOOKUPS].1.contains("Do not ask for more groups."));
    }
}
