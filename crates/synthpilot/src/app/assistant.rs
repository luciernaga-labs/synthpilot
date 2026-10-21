use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use synthpilot_assistant::{
    Assistant, AssistantError, Config, Context, HttpTransport, PartContext, Proposal, Resolved,
    resolve,
};
use synthpilot_core::domain::ParameterChange;

use super::state::{Action, EFFECTS, Workspace};

pub(crate) const IDEAS: [&str; 5] = [
    "Crystal Rhodes",
    "Bright piano",
    "Warm pad",
    "More space",
    "Darker and softer",
];

pub(crate) enum Setup {
    Ready(Arc<Assistant>),
    Missing,
    Invalid(String),
}

pub(crate) struct Row {
    pub(crate) resolved: Resolved,
    pub(crate) selected: bool,
}

pub(crate) struct Plan {
    pub(crate) request: String,
    pub(crate) summary: String,
    pub(crate) rows: Vec<Row>,
}

impl Plan {
    pub(crate) fn chosen(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| row.selected && row.resolved.is_applicable())
            .count()
    }
}

struct Pending {
    part: u8,
    request: String,
    reply: Receiver<Result<Proposal, AssistantError>>,
}

pub(crate) struct AssistantPanel {
    setup: Setup,
    pub(crate) prompt: String,
    pending: Option<Pending>,
    plan: Option<Plan>,
    undo: Vec<ParameterChange>,
    message: Option<String>,
    error: Option<String>,
}

impl AssistantPanel {
    pub(crate) const fn new(setup: Setup) -> Self {
        Self {
            setup,
            prompt: String::new(),
            pending: None,
            plan: None,
            undo: Vec::new(),
            message: None,
            error: None,
        }
    }

    pub(crate) fn from_env() -> Self {
        let setup = match Config::from_env() {
            Ok(Some(config)) => Setup::Ready(Arc::new(Assistant::new(
                config,
                Box::new(HttpTransport::new()),
            ))),
            Ok(None) => Setup::Missing,
            Err(error) => Setup::Invalid(error.to_string()),
        };
        Self::new(setup)
    }

    pub(crate) const fn setup(&self) -> &Setup {
        &self.setup
    }

    pub(crate) fn model(&self) -> Option<String> {
        match &self.setup {
            Setup::Ready(assistant) => {
                let config = assistant.config();
                Some(format!("{} · {}", config.provider.name(), config.model))
            }
            Setup::Missing | Setup::Invalid(_) => None,
        }
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.pending.is_some()
    }

    pub(crate) fn waiting_for(&self) -> Option<&str> {
        self.pending
            .as_ref()
            .map(|pending| pending.request.as_str())
    }

    pub(crate) const fn plan(&self) -> Option<&Plan> {
        self.plan.as_ref()
    }

    pub(crate) fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub(crate) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub(crate) fn ask(&mut self, workspace: &Workspace) {
        let request = self.prompt.trim().to_owned();
        let Setup::Ready(assistant) = &self.setup else {
            return;
        };
        if request.is_empty() || self.pending.is_some() {
            return;
        }
        let part = workspace.part();
        let Some(context) = context(workspace, part) else {
            self.error = Some("Read a Performance from the keyboard first.".to_owned());
            return;
        };
        let catalog = context.describe();
        let listings = context.listings();
        let assistant = Arc::clone(assistant);
        let question = request.clone();
        let (sender, reply) = mpsc::channel();
        thread::spawn(move || {
            let _ = sender.send(assistant.propose(&catalog, &listings, &question));
        });
        self.pending = Some(Pending {
            part,
            request,
            reply,
        });
        self.plan = None;
        self.message = None;
        self.error = None;
    }

    pub(crate) fn poll(&mut self, workspace: &Workspace) {
        let Some(pending) = &self.pending else {
            return;
        };
        let result = match pending.reply.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                Err(AssistantError::Network("the request stopped".to_owned()))
            }
        };
        let Some(pending) = self.pending.take() else {
            return;
        };
        match result {
            Ok(proposal) => {
                let Some(context) = context(workspace, pending.part) else {
                    self.error = Some("The Performance changed while waiting.".to_owned());
                    return;
                };
                let rows: Vec<Row> = resolve(&context, &proposal)
                    .into_iter()
                    .map(|resolved| Row {
                        selected: resolved.is_applicable(),
                        resolved,
                    })
                    .collect();
                self.plan = Some(Plan {
                    request: pending.request,
                    summary: proposal.summary,
                    rows,
                });
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    pub(crate) fn toggle(&mut self, index: usize) {
        if let Some(row) = self
            .plan
            .as_mut()
            .and_then(|plan| plan.rows.get_mut(index))
            .filter(|row| row.resolved.is_applicable())
        {
            row.selected = !row.selected;
        }
    }

    pub(crate) fn apply(&mut self, workspace: &Workspace) -> Vec<Action> {
        let Some(plan) = self.plan.take() else {
            return Vec::new();
        };
        let chosen = || {
            plan.rows
                .iter()
                .filter(|row| row.selected && row.resolved.is_applicable())
        };
        let loads: Vec<Action> = chosen()
            .filter_map(|row| row.resolved.load)
            .map(|(part, number)| Action::LoadPreset { part, number })
            .collect();
        let mut changes: Vec<ParameterChange> =
            chosen().filter_map(|row| row.resolved.change).collect();
        changes.sort_by_key(|change| !workspace.reshapes(Action::Apply(*change)));
        self.undo = changes
            .iter()
            .filter_map(|change| {
                workspace
                    .value_at(change)
                    .map(|value| ParameterChange::new(change.parameter, change.location, value))
            })
            .collect();
        let applied = match changes.len() {
            0 => None,
            1 => Some("Applied 1 change.".to_owned()),
            count => Some(format!("Applied {count} changes.")),
        };
        let sent = match loads.len() {
            0 => None,
            1 => Some("Loading 1 sound; undo doesn't bring back the previous sound.".to_owned()),
            count => Some(format!(
                "Loading {count} sounds; undo doesn't bring back the previous sounds."
            )),
        };
        let total = changes.len() + loads.len();
        let mut message: Vec<String> = applied.into_iter().chain(sent).collect();
        message.push(if total == 1 {
            "Play the keyboard to hear it.".to_owned()
        } else {
            "Play the keyboard to hear them.".to_owned()
        });
        self.message = Some(message.join(" "));
        changes
            .into_iter()
            .map(Action::Apply)
            .chain(loads)
            .collect()
    }

    pub(crate) fn undo(&mut self) -> Vec<Action> {
        let actions: Vec<Action> = self.undo.drain(..).map(Action::Apply).collect();
        if !actions.is_empty() {
            self.message = Some("Put the previous values back.".to_owned());
        }
        actions
    }

    pub(crate) fn discard(&mut self) {
        self.plan = None;
    }
}

pub(crate) fn context(workspace: &Workspace, part: u8) -> Option<Context<'_>> {
    let sound = workspace.sound()?;
    let device = workspace.device();
    let performance = sound
        .common()
        .zip(device.common())
        .map(|(values, engine)| PartContext {
            number: EFFECTS,
            engine,
            part: values,
        });
    let parts = (1..=sound.part_count())
        .filter_map(|number| {
            let values = sound.part(number)?;
            let engine = device.engine(values.engine())?;
            Some(PartContext {
                number,
                engine,
                part: values,
            })
        })
        .collect();
    Some(Context {
        device: device.display_name(),
        sound: sound.name(),
        performance,
        parts,
        selected: (part != EFFECTS).then_some(part),
        presets: device.presets(),
        part_count: usize::from(device.part_count()),
    })
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use synthpilot_assistant::{Provider, Transport};
    use synthpilot_core::devices::modx_plus::{awm2, effects, part};
    use synthpilot_core::devices::{self, modx_plus};
    use synthpilot_core::domain::{Location, Sound};

    use super::*;

    struct Reply(Result<(u16, String), String>);

    impl Transport for Reply {
        fn post(
            &self,
            _url: &str,
            _headers: &[(&'static str, String)],
            _body: &str,
        ) -> Result<(u16, String), String> {
            self.0.clone()
        }
    }

    fn workspace() -> Workspace {
        let mut workspace = Workspace::new(devices::load(modx_plus::DEVICE_ID).unwrap());
        let sound = {
            let device = workspace.device();
            Sound::with_engine(device, device.engine(awm2::ENGINE_ID).unwrap())
        };
        workspace.load(sound);
        workspace
    }

    fn panel(content: &str) -> AssistantPanel {
        let escaped = serde_json_escape(content);
        let body = format!(r#"{{"choices":[{{"message":{{"content":"{escaped}"}}}}]}}"#);
        ready(Ok((200, body)))
    }

    fn ready(reply: Result<(u16, String), String>) -> AssistantPanel {
        let config = Config {
            provider: Provider::OpenAi,
            model: "gpt-test".to_owned(),
            api_key: "k".to_owned(),
            base_url: "http://local".to_owned(),
        };
        AssistantPanel::new(Setup::Ready(Arc::new(Assistant::new(
            config,
            Box::new(Reply(reply)),
        ))))
    }

    fn serde_json_escape(text: &str) -> String {
        text.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    }

    fn wait(panel: &mut AssistantPanel, workspace: &Workspace) {
        let started = Instant::now();
        while panel.is_busy() && started.elapsed() < Duration::from_secs(5) {
            panel.poll(workspace);
            thread::sleep(Duration::from_millis(5));
        }
        assert!(!panel.is_busy());
    }

    const PLAN: &str = r#"{"summary":"Bell-like and wide","changes":[
        {"parameter":"part.reverb-send","element":null,"value":60,"reason":"space"},
        {"parameter":"effects.variation-type","value":17,"reason":"chorus"},
        {"parameter":"awm2.element-level","element":1,"value":110,"reason":"level"},
        {"parameter":"awm2.filter-type","element":1,"value":5,"reason":"tine"}
    ]}"#;

    #[test]
    fn proposes_reviews_applies_and_undoes() {
        let workspace = workspace();
        let mut panel = panel(PLAN);
        assert_eq!(panel.model().as_deref(), Some("OpenAI · gpt-test"));
        panel.prompt = "  crystal rhodes ".to_owned();
        panel.ask(&workspace);
        assert_eq!(panel.waiting_for(), Some("crystal rhodes"));
        panel.ask(&workspace);
        wait(&mut panel, &workspace);
        assert_eq!(panel.error(), None);

        let plan = panel.plan().unwrap();
        assert_eq!(plan.request, "crystal rhodes");
        assert_eq!(plan.summary, "Bell-like and wide");
        assert_eq!(plan.rows.len(), 4);
        assert_eq!(plan.chosen(), 3);
        assert!(!plan.rows[3].selected);

        panel.toggle(2);
        panel.toggle(3);
        assert_eq!(panel.plan().unwrap().chosen(), 2);

        let actions = panel.apply(&workspace);
        assert_eq!(
            actions,
            [
                Action::Apply(ParameterChange::new(
                    effects::VARIATION_TYPE,
                    Location::performance(),
                    17
                )),
                Action::Apply(ParameterChange::new(
                    part::REVERB_SEND,
                    Location::part(1),
                    60
                )),
            ]
        );
        assert!(panel.plan().is_none());
        assert_eq!(
            panel.message(),
            Some("Applied 2 changes. Play the keyboard to hear them.")
        );
        assert!(panel.can_undo());
        assert_eq!(
            panel.undo(),
            [
                Action::Apply(ParameterChange::new(
                    effects::VARIATION_TYPE,
                    Location::performance(),
                    15
                )),
                Action::Apply(ParameterChange::new(
                    part::REVERB_SEND,
                    Location::part(1),
                    12
                )),
            ]
        );
        assert!(!panel.can_undo());
        assert_eq!(panel.undo(), Vec::<Action>::new());
        assert_eq!(panel.apply(&workspace), Vec::<Action>::new());
    }

    #[test]
    fn proposed_sounds_load_after_the_changes() {
        let workspace = workspace();
        let mut panel = panel(
            r#"{"summary":"Bells on top","load":[{"part":2,"sound":955,"reason":"bells"}],"changes":[
                {"parameter":"part.volume","part":1,"value":90,"reason":"room"},
                {"parameter":"part.volume","part":2,"value":80,"reason":"too soon"}
            ]}"#,
        );
        panel.prompt = "add bells".to_owned();
        panel.ask(&workspace);
        wait(&mut panel, &workspace);
        let plan = panel.plan().unwrap();
        assert_eq!(plan.rows[0].resolved.name, "Part 2 · Sound");
        assert_eq!(plan.rows[0].resolved.from.as_deref(), Some("Empty"));
        assert_eq!(
            plan.rows[0].resolved.to.as_deref(),
            Some("955 Gamelan (Chromatic Perc/Bell)")
        );
        assert!(plan.rows[2].resolved.problem.is_some());
        assert_eq!(plan.chosen(), 2);
        let actions = panel.apply(&workspace);
        assert_eq!(
            actions,
            [
                Action::Apply(ParameterChange::new(part::VOLUME, Location::part(1), 90)),
                Action::LoadPreset {
                    part: 2,
                    number: 955
                },
            ]
        );
        assert_eq!(
            panel.message(),
            Some(
                "Applied 1 change. Loading 1 sound; undo doesn't bring back the previous sound. Play the keyboard to hear them."
            )
        );
    }

    #[test]
    fn single_changes_and_discarding() {
        let workspace = workspace();
        let mut panel = panel(
            r#"{"summary":"","changes":[{"parameter":"part.volume","value":90,"reason":""}]}"#,
        );
        panel.prompt = "quieter".to_owned();
        panel.ask(&workspace);
        wait(&mut panel, &workspace);
        panel.discard();
        assert!(panel.plan().is_none());
        panel.ask(&workspace);
        wait(&mut panel, &workspace);
        assert_eq!(panel.apply(&workspace).len(), 1);
        assert_eq!(
            panel.message(),
            Some("Applied 1 change. Play the keyboard to hear it.")
        );
    }

    #[test]
    fn errors_are_shown() {
        let workspace = workspace();
        let mut panel = ready(Err("offline".to_owned()));
        panel.prompt = "pad".to_owned();
        panel.ask(&workspace);
        wait(&mut panel, &workspace);
        assert_eq!(
            panel.error(),
            Some("Couldn't reach the AI service: offline")
        );

        let empty = Workspace::new(devices::load(modx_plus::DEVICE_ID).unwrap());
        panel.ask(&empty);
        assert_eq!(
            panel.error(),
            Some("Read a Performance from the keyboard first.")
        );
        assert!(context(&empty, 1).is_none());
    }

    #[test]
    fn nothing_happens_without_setup_or_text() {
        let workspace = workspace();
        let mut missing = AssistantPanel::new(Setup::Missing);
        missing.prompt = "pad".to_owned();
        missing.ask(&workspace);
        assert!(!missing.is_busy());
        assert!(missing.model().is_none());
        assert!(matches!(missing.setup(), Setup::Missing));
        let invalid = AssistantPanel::new(Setup::Invalid("bad".to_owned()));
        assert!(invalid.model().is_none());

        let mut blank = panel("{}");
        blank.ask(&workspace);
        assert!(!blank.is_busy());
        blank.poll(&workspace);
        blank.toggle(0);
    }

    #[test]
    fn the_effects_page_only_offers_effects() {
        let mut workspace = workspace();
        workspace.apply(Action::SelectPart(EFFECTS));
        let context = context(&workspace, EFFECTS).unwrap();
        assert!(context.performance.is_some());
        assert!(context.selected_part().is_none());
        assert_eq!(context.parts.len(), 1);
        assert_eq!(context.device, "Yamaha MODX+");
        assert_eq!(IDEAS[0], "Crystal Rhodes");
    }
}
