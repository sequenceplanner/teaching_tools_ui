use iced::{
    button, executor, window, Alignment, Application, Button, Column, Command, Element, Settings,
    Text,
};
use r2r::std_srvs::srv::Trigger;
use std::sync::{Arc, Mutex};

pub static NODE_ID: &'static str = "teaching_tools_ui";

// #[derive(Debug)]
struct TeachingToolsUI {
    reset_button: button::State,
    match_button: button::State,
    reset_ghost_client: Arc<Mutex<r2r::Client<Trigger::Service>>>,
    reset_marker_client: Arc<Mutex<r2r::Client<Trigger::Service>>>,
    match_ghost_client: Arc<Mutex<r2r::Client<Trigger::Service>>>,
    // waiting_for_reset_ghost_server: impl Future<Output = r2r::Result<()>>
}

pub fn main() -> iced::Result {
    TeachingToolsUI::run(Settings {
        antialiasing: true,
        window: window::Settings {
            position: window::Position::Centered,
            size: (200, 200),
            ..window::Settings::default()
        },
        ..Settings::default()
    })
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Empty,
    Reset,
    Match,
}

impl Application for TeachingToolsUI {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (TeachingToolsUI, Command<Message>) {
        let ctx = r2r::Context::create().expect("could not create context");
        let mut node = r2r::Node::create(ctx, NODE_ID, "").expect("...");
        let reset_ghost = node
            .create_client::<Trigger::Service>("reset_ghost")
            .expect("could not create reset ghost client");
        // let waiting_for_reset_ghost_server = node.is_available(&reset_ghost);
        let reset_ghost_client = Arc::new(Mutex::new(reset_ghost));
        let reset_marker_client = Arc::new(Mutex::new(
            node.create_client::<Trigger::Service>("reset_teaching_marker")
                .expect("could not create reset marker client"),
        ));
        let match_ghost_client = Arc::new(Mutex::new(
            node.create_client::<Trigger::Service>("match_ghost")
                .expect("could not create match ghost client"),
        ));

        let _handle = std::thread::spawn(move || loop {
            node.spin_once(std::time::Duration::from_millis(100));
        });

        (
            TeachingToolsUI {
                reset_button: button::State::new(),
                match_button: button::State::new(),
                reset_ghost_client: reset_ghost_client.clone(),
                reset_marker_client: reset_marker_client.clone(),
                match_ghost_client: match_ghost_client.clone(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Reset ghost service")
    }

    fn update(&mut self, message: Message) -> iced::Command<Message> {
        match message {
            Message::Reset => {
                Command::perform(
                    reset_ghost_and_marker(
                        self.reset_ghost_client.clone(),
                        self.reset_marker_client.clone(),
                        // waiting_for_reset_ghost_server
                    ),
                    |_| Message::Empty,
                )
            }
            Message::Match => {
                Command::perform(match_ghost(self.match_ghost_client.clone()), |_| {
                    Message::Empty
                })
            }
            Message::Empty => Command::none(),
        }
    }

    fn view(&mut self) -> Element<Message> {
        Column::new()
            .padding(20)
            .align_items(Alignment::Center)
            .push(
                Button::new(&mut self.reset_button, Text::new("reset ghost"))
                    .on_press(Message::Reset),
            )
            .push(
                Button::new(&mut self.match_button, Text::new("match ghost"))
                    .on_press(Message::Match),
            )
            .into()
    }
}

// ask the ghost reset service to put the ghost in its initial pose
async fn reset_ghost_and_marker(
    ghost_client: Arc<Mutex<r2r::Client<Trigger::Service>>>,
    marker_client: Arc<Mutex<r2r::Client<Trigger::Service>>>,
    // waiting: impl Future<Output = r2r::Result<()>>,
) -> Option<()> {
    // r2r::log_warn!(NODE_ID, "Waiting for tf Lookup service...");
    // waiting.await;
    // r2r::log_info!(NODE_ID, "tf Lookup Service available.");

    let ghost_request_msg = Trigger::Request {};
    let marker_request_msg = Trigger::Request {};

    let ghost_request = ghost_client
        .lock()
        .unwrap()
        .request(&ghost_request_msg)
        .expect("Could not send reset ghost request.");

    r2r::log_info!(NODE_ID, "Request to reset the ghost sent.");

    let marker_request = marker_client
        .lock()
        .unwrap()
        .request(&marker_request_msg)
        .expect("Could not send reset ghost request.");

    // are the ghost and the marker even responding or not?
    r2r::log_info!(NODE_ID, "Request to reset the ghost sent.");
    let ghost_response = ghost_request.await.expect("asdf");
    let marker_response = marker_request.await.expect("asdf");

    match ghost_response.success {
        true => match marker_response.success {
            true => {
                r2r::log_info!(NODE_ID, "Ghost and marker are reset.");
                Some(())
            }
            false => {
                r2r::log_error!(NODE_ID, "Couldn't reset marker.",);
                None
            }
        },
        false => {
            r2r::log_error!(NODE_ID, "Couldn't reset ghost.",);
            None
        }
    }
}

// ask the robot to go where the ghost is
async fn match_ghost(client: Arc<Mutex<r2r::Client<Trigger::Service>>>) -> Option<()> {
    let request_msg = Trigger::Request {};

    let request = client
        .lock()
        .unwrap()
        .request(&request_msg)
        .expect("Could not send reset ghost request.");

    r2r::log_info!(NODE_ID, "Request to reset the ghost sent.");

    // are the ghost and the marker even responding or not?
    r2r::log_info!(NODE_ID, "Request to reset the ghost sent.");
    let response = request.await.expect("asdf");

    match response.success {
        true => {
            r2r::log_info!(NODE_ID, "Ghost and marker are reset.");
            Some(())
        }
        false => {
            r2r::log_error!(NODE_ID, "Couldn't reset marker.",);
            None
        }
    }
}
