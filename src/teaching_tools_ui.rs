use futures::Stream;
use futures::StreamExt;
use iced::{
    button, executor, window, Alignment, Application, Button, Column, Command, Element, Settings,
    Text,
};
use r2r::sensor_msgs::msg::JointState;
use r2r::std_msgs::msg::Header;
use r2r::std_srvs::srv::Trigger;
use r2r::ur_controller_msgs::action::URControl;
use r2r::ActionClient;
use r2r::QosProfile;
use std::sync::{Arc, Mutex};

pub static NODE_ID: &'static str = "teaching_tools_ui";

// #[derive(Debug)]
struct TeachingToolsUI {
    reset_button: button::State,
    match_button: button::State,
    reset_ghost_client: Arc<Mutex<r2r::Client<Trigger::Service>>>,
    reset_marker_client: Arc<Mutex<r2r::Client<Trigger::Service>>>,
    match_ghost_client:
        Arc<Mutex<ActionClient<r2r::ur_controller_msgs::action::URControl::Action>>>,
    ghost_state: Arc<Mutex<JointState>>,
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

// instead of the match service and so, listen to the ghost joint state here and
// send that as the move request using 'use_joint_state'
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
        // listen to the current teaching ghost pose to go to (in teaching mode?)
        let ghost_state_subscriber = node
            .subscribe::<JointState>("ghost/joint_states", QosProfile::default())
            .expect("could not joint state subscriber");
        let match_ghost_client = Arc::new(Mutex::new(
            node.create_action_client::<URControl::Action>("ur_control")
                .expect("could not create ur control client"),
        ));

        // initialize the ghost joint state
        let ghost_joint_state = Arc::new(Mutex::new(JointState {
            header: Header {
                ..Default::default()
            },
            ..Default::default()
        }));

        let ghost_joint_state_clone_1 = ghost_joint_state.clone();
        tokio::task::spawn(async move {
            match ghost_subscriber_callback(ghost_state_subscriber, &ghost_joint_state_clone_1)
                .await
            {
                Ok(()) => (),
                Err(e) => r2r::log_error!(NODE_ID, "Joint state subscriber failed with {}.", e),
            };
        });

        let _handle = std::thread::spawn(move || loop {
            node.spin_once(std::time::Duration::from_millis(100));
        });

        let ghost_joint_state_clone_2 = ghost_joint_state.clone();
        (
            TeachingToolsUI {
                reset_button: button::State::new(),
                match_button: button::State::new(),
                reset_ghost_client: reset_ghost_client.clone(),
                reset_marker_client: reset_marker_client.clone(),
                match_ghost_client: match_ghost_client.clone(),
                ghost_state: ghost_joint_state_clone_2,
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
            Message::Match => Command::perform(
                match_ghost(self.match_ghost_client.clone(), self.ghost_state.clone()),
                |_| Message::Empty,
            ),
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

// subscribe to the current joint pose of the ghost robot
async fn ghost_subscriber_callback(
    mut subscriber: impl Stream<Item = JointState> + Unpin,
    ghost_joint_state: &Arc<Mutex<JointState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match subscriber.next().await {
            Some(msg) => {
                let mut new_joint_state = ghost_joint_state.lock().unwrap().clone();
                new_joint_state.position = msg.position;
                *ghost_joint_state.lock().unwrap() = new_joint_state;
            }
            None => {
                r2r::log_error!(NODE_ID, "Subscriber did not get the message?");
                ()
            }
        }
    }
}

// ask the ghost reset service to put the ghost in its initial pose
async fn reset_ghost_and_marker(
    ghost_client: Arc<Mutex<r2r::Client<Trigger::Service>>>,
    marker_client: Arc<Mutex<r2r::Client<Trigger::Service>>>,
    // waiting: impl Future<Output = r2r::Result<()>>,
) -> Option<()> {

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
async fn match_ghost(
    client: Arc<Mutex<ActionClient<r2r::ur_controller_msgs::action::URControl::Action>>>,
    joint_state: Arc<Mutex<JointState>>,
) -> Option<()> {
    let request_msg = Trigger::Request {};
    let joint_state_local = joint_state.lock().unwrap().clone();
    let goal = URControl::Goal {
        command: "move_j".to_string(),
        use_joint_positions: true,
        joint_positions: joint_state_local,
        velocity: 0.1,
        acceleration: 0.1,
        ..Default::default()
    };

    let c = client.lock().unwrap().clone();

    let (_goal, result, _feedback) = match c.send_goal_request(goal) {
        Ok(x) => match x.await {
            Ok(y) => y,
            Err(_) => {
                r2r::log_info!(NODE_ID, "Could not send goal request.");
                return None;
            }
        },
        Err(_) => {
            r2r::log_info!(NODE_ID, "Did not get goal.");
            return None;
        }
    };

    match result.await {
        Ok((status, msg)) => match status {
            r2r::GoalStatus::Aborted => {
                r2r::log_info!(NODE_ID, "Goal succesfully aborted with: {:?}", msg);
                Some(())
            }
            _ => {
                r2r::log_info!(
                    NODE_ID,
                    "Executing the Simple Robot Simulator Command succeeded."
                );
                Some(())
            }
        },
        Err(e) => {
            r2r::log_error!(
                NODE_ID,
                "Simple Robot Simulator Action failed with: {:?}",
                e,
            );
            None
        }
    }
}