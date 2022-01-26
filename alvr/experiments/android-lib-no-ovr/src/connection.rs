use crate::{
    audio,
    buffer_queue,
    common::{
        ConnectionError, ConnectionEvent, ConnectionSettings,
        FfrParam,
    },
    device::{self, Device},
    legacy_packets::*,
    legacy_stream::StreamHandler,
    latency_controller,
    packet,
    util,
};
use alvr_common::{
    glam::{Quat, Vec2, Vec3},
    prelude::*,
    Haptics,
    MotionData,
    TrackedDeviceType,
    ALVR_NAME, ALVR_VERSION,
};
use alvr_session::*;
use alvr_sockets::*;
use bincode;
use futures::future::BoxFuture;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json as json;
use settings_schema::Switch;
use std::{
    mem,
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};
use tokio::{
    net::UdpSocket,
    runtime::Runtime,
    sync::mpsc as tmpsc,
    sync::Notify,
    task::JoinHandle,
    time::{self, Instant},
};

const CHANNEL_BUFFER_SIZE: usize = 128;

const RETRY_CONNECT_MIN_INTERVAL: Duration = Duration::from_secs(1);
const CONTROL_CONNECT_RETRY_PAUSE: Duration = Duration::from_millis(500);
const CLIENT_HANDSHAKE_RESEND_INTERVAL: Duration = Duration::from_secs(1);
const SET_UP_STREAM_TIMEOUT: Duration = Duration::from_secs(5);
const PLAYSPACE_SYNC_INTERVAL: Duration = Duration::from_millis(500);
const NETWORK_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(1);
const TRACKING_INTERVAL: f32 = 1. / 360.;

// const CLEANUP_PAUSE: Duration = Duration::from_millis(500);

static MAYBE_RUNTIME: Lazy<Mutex<Option<Runtime>>> = Lazy::new(|| Mutex::new(None));
static MAYBE_RENDERED_SENDER: Lazy<Mutex<Option<tmpsc::UnboundedSender<u64>>>> = Lazy::new(|| Mutex::new(None));
static MAYBE_OBSERVER: Lazy<Mutex<Option<Box<dyn ConnectionObserver>>>> = Lazy::new(|| Mutex::new(None));
static IDR_REQUEST_NOTIFIER: Lazy<Notify> = Lazy::new(|| Notify::new());

pub trait ConnectionObserver: Send {
    fn on_event_occurred(&self, event: ConnectionEvent) -> StrResult;
}

pub fn set_observer(observer: Box<dyn ConnectionObserver>) {
    *MAYBE_OBSERVER.lock() = Some(observer);
}

pub fn connect(
    device: &'static Device,
    private_identity: &'static PrivateIdentity,
) -> StrResult<Option<JoinHandle<()>>> {
    let mut maybe_runtime = MAYBE_RUNTIME.lock();

    if (*maybe_runtime).is_some() {
        warn!("The connection is already connected.");
        return Ok(None);
    }

    let runtime = trace_err!(Runtime::new())?;
    let handle = runtime.spawn(connection_lifecycle_loop(device, private_identity));
    *maybe_runtime = Some(runtime);

    Ok(Some(handle))
}

pub fn disconnect() {
    let mut maybe_runtime = MAYBE_RUNTIME.lock();

    if (*maybe_runtime).is_none() {
        warn!("The connection is not connected.");
        return;
    }

    buffer_queue::terminate_loop();

    // shutdown and wait for tasks to finish
    drop(maybe_runtime.take());

    info!("The connection has been disconnected.")
}

fn notify_event(event: ConnectionEvent) {
    info!("{:?}", event);
    MAYBE_OBSERVER.lock().as_ref().map(
        |observer| observer.on_event_occurred(event)
    );
}

async fn connection_lifecycle_loop(
    device: &'static Device,
    identity: &'static PrivateIdentity,
) {
    notify_event(ConnectionEvent::Initial);

    loop {
        tokio::join!(
            async {
                match connection_pipeline(device, &identity).await {
                    Err(error) => {
                        notify_event(ConnectionEvent::Error { error });
                        time::sleep(RETRY_CONNECT_MIN_INTERVAL).await;
                        notify_event(ConnectionEvent::Initial);
                    },
                    Ok(_) => ()
                }
                buffer_queue::terminate_loop();
                // TODO Do I need the following?
                // // let any running task or socket shutdown
                // time::sleep(CLEANUP_PAUSE).await; // 500 msec
            },
            time::sleep(RETRY_CONNECT_MIN_INTERVAL),
        );
    }
}

async fn connection_pipeline(
    device: &'static Device,
    identity: &PrivateIdentity,
) -> Result<(), ConnectionError> {
    let hostname = &identity.hostname;

    let handshake_packet = ClientHandshakePacket {
        alvr_name: ALVR_NAME.into(),
        version: ALVR_VERSION.clone(),
        device_name: device.name.clone(),
        hostname: hostname.into(),
        reserved1: "".into(),
        reserved2: "".into(),
    };

    let (mut proto_socket, server_ip) = tokio::select! {
        pair = control_connect_loop() => pair,
        res = announce_client_loop(handshake_packet) => {
            assert!(matches!(res, Err(_)));
            return Err(res.unwrap_err());
        },
    };

    notify_event(ConnectionEvent::ServerFound { ipaddr: server_ip });

    let headset_info = HeadsetInfoPacket {
        recommended_eye_width: device.recommended_eye_width,
        recommended_eye_height: device.recommended_eye_height,
        available_refresh_rates: device.available_refresh_rates.to_vec(),
        preferred_refresh_rate: device.preferred_refresh_rate,
        reserved: format!("{}", *ALVR_VERSION),
    };

    proto_socket.send(&(headset_info, server_ip)).await?;
    let config_packet = proto_socket.recv::<ClientConfigPacket>().await?;

    let (mut control_sender, mut control_receiver) =
        proto_socket.split::<ClientControlPacket, ServerControlPacket>();

    match control_receiver.recv().await {
        Ok(ServerControlPacket::StartStream) => {
            notify_event(ConnectionEvent::StreamStart);
        }
        Ok(ServerControlPacket::Restarting) => {
            notify_event(ConnectionEvent::ServerRestart);
            return Ok(());
        }
        Ok(_) => {
            return Err(ConnectionError::SystemError {
                cause: "Unexpected packet".into()
            });
        }
        Err(e) => {
            return Err(ConnectionError::ServerDisconnected {
                cause: format!("{} {}", trace_str!(), e)
            });
        }
    }

    let settings = {
        let session_desc_json = trace_err!(json::from_str(&config_packet.session_desc))?;
        let mut session_desc = SessionDesc::default();
        session_desc.merge_from_json(&session_desc_json)?;
        session_desc.to_settings()
    };

    let stream_port = settings.connection.stream_port;

    let stream_socket_builder =
        StreamSocketBuilder::listen_for_server(
            stream_port,
            settings.connection.stream_protocol,
        ).await?;

    if let Err(e) = control_sender.send(&ClientControlPacket::StreamReady).await {
        return Err(ConnectionError::ServerDisconnected {
            cause: format!("{} {}", trace_str!(), e)
        });
    }

    let stream_socket = tokio::select! {
        res = stream_socket_builder.accept_from_server(server_ip, stream_port) => res?,
        _ = time::sleep(SET_UP_STREAM_TIMEOUT) => {
            return Err(ConnectionError::TimeoutSetUpStream);
        }
    };

    notify_event(ConnectionEvent::Connected {
        settings: to_connection_settings(&config_packet, &settings.video)
    });

    // let is_connected = Arc::new(AtomicBool::new(true));
    // let _stream_guard = StreamCloseGuard {
    //     is_connected: Arc::clone(&is_connected),
    // };

    let tracking_clientside_prediction = match &settings.headset.controllers {
        Switch::Enabled(controllers) => controllers.clientside_prediction,
        Switch::Disabled => false,
    };

    let (legacy_receive_data_sender, legacy_receive_data_receiver) = tmpsc::unbounded_channel();

    // senders are used by the sync context.
    let (time_sync_sender, time_sync_receiver) = tmpsc::unbounded_channel();
    let (video_error_report_sender, video_error_report_receiver) = tmpsc::unbounded_channel();

    let legacy_receive_loop = legacy_receive_loop(
        legacy_receive_data_receiver,
        time_sync_sender.clone(),
        video_error_report_sender.clone(),
        settings.video.codec,
        settings.connection.enable_fec,
    );

    let video_receive_loop = video_receive_loop(
        stream_socket.subscribe_to_stream::<VideoFrameHeaderPacket>(VIDEO).await?,
        legacy_receive_data_sender.clone(),
    );

    let haptics_receive_loop = haptics_receive_loop(
        stream_socket.subscribe_to_stream::<Haptics<TrackedDeviceType>>(HAPTICS).await?,
        legacy_receive_data_sender.clone(),
    );

    let (control_send_data_sender, control_send_data_receiver) = tmpsc::channel(CHANNEL_BUFFER_SIZE);

    let (rendered_sender, rendered_receiver) = tmpsc::unbounded_channel();
    *MAYBE_RENDERED_SENDER.lock() = Some(rendered_sender);

    let tracking_loop = tracking_loop(
        stream_socket.request_stream(INPUT).await?
    );
    let time_sync_send_loop = time_sync_send_loop(
        rendered_receiver,
        control_send_data_sender.clone(),
    );
    let time_sync_send_back_loop = time_sync_send_back_loop(
        time_sync_receiver,
        control_send_data_sender.clone(),
    );
    let video_error_report_send_loop = video_error_report_send_loop(
        video_error_report_receiver,
        control_send_data_sender.clone(),
    );
    let playspace_sync_loop = playspace_sync_loop(
        control_send_data_sender.clone()
    );
    let request_idr_loop = request_idr_loop(
        control_send_data_sender.clone()
    );

    let keepalive_sender_loop = keepalive_sender_loop(
        control_send_data_sender.clone()
    );

    let control_send_loop = control_send_loop(
        control_send_data_receiver,
        control_sender,
    );

    let control_receive_loop = control_receive_loop(
        control_receiver,
        legacy_receive_data_sender.clone(),
    );

    let game_audio_loop = game_audio_loop(
        stream_socket.subscribe_to_stream(AUDIO).await?,
        settings.audio.game_audio,
        config_packet.game_audio_sample_rate,
    );
    let microphone_loop = microphone_loop(
        stream_socket.request_stream(AUDIO).await?,
        settings.audio.microphone,
    );

    let receive_loop = async move {
        stream_socket.receive_loop().await
    };

    // Run many tasks concurrently. Threading is managed by the runtime, for best performance.
    (tokio::select! {
        res = spawn_cancelable(receive_loop) => {
            if let Err(e) = res {
                return Err(ConnectionError::ServerDisconnected {
                    cause: format!("{}\n{}", e, trace_str!())
                });
            }
            info!("receive_loop finished");
            Ok(())
        },
        res = spawn_cancelable(game_audio_loop) => trace_err!(res),
        res = spawn_cancelable(microphone_loop) => trace_err!(res),
        res = spawn_cancelable(legacy_receive_loop) => trace_err!(res),
        res = spawn_cancelable(video_receive_loop) => trace_err!(res),
        res = spawn_cancelable(haptics_receive_loop) => trace_err!(res),
        res = spawn_cancelable(tracking_loop) => trace_err!(res),
        res = spawn_cancelable(time_sync_send_loop) => trace_err!(res),
        res = spawn_cancelable(time_sync_send_back_loop) => trace_err!(res),
        res = spawn_cancelable(video_error_report_send_loop) => trace_err!(res),
        res = spawn_cancelable(playspace_sync_loop) => trace_err!(res),
        res = spawn_cancelable(request_idr_loop) => trace_err!(res),
        res = buffer_queue::buffer_coordination_loop() => trace_err!(res)?,

        // keep these loops on the current task
        res = keepalive_sender_loop => trace_err!(res),
        res = control_send_loop => trace_err!(res),
        res = control_receive_loop => trace_err!(res),
    })?;

    info!("loop finished");
    Ok(())
}

fn to_connection_settings(
    config_packet: &ClientConfigPacket,
    video_desc: &VideoDesc,
) -> ConnectionSettings {
    let ffr_param = if let Switch::Enabled(ref foveation_vars) = video_desc.foveated_rendering {
        Some(FfrParam {
            eye_width: config_packet.eye_resolution_width as i32,
            eye_height: config_packet.eye_resolution_height as i32,
            center_size_x: foveation_vars.center_size_x,
            center_size_y: foveation_vars.center_size_y,
            center_shift_x: foveation_vars.center_shift_x,
            center_shift_y: foveation_vars.center_shift_y,
            edge_ratio_x: foveation_vars.edge_ratio_x,
            edge_ratio_y: foveation_vars.edge_ratio_y,
        })
    } else {
        None
    };
    ConnectionSettings {
        fps: config_packet.fps,
        codec: video_desc.codec.into(),
        realtime: video_desc.client_request_realtime_decoder,
        dashboard_url: config_packet.dashboard_url.clone(),
        ffr_param,
    }
}

async fn control_connect_loop() -> (ProtoControlSocket, IpAddr) {
    loop {
        if let Ok(socket_ipaddr_pair) = ProtoControlSocket::connect_to(PeerType::Server).await {
            break socket_ipaddr_pair;
        }
        time::sleep(CONTROL_CONNECT_RETRY_PAUSE).await;
    }
}

async fn announce_client_loop(
    handshake_packet: ClientHandshakePacket,
) -> Result<(), ConnectionError> {
    let mut handshake_socket =
        trace_err!(UdpSocket::bind((LOCAL_IP, CONTROL_PORT)).await)?;
    trace_err!(handshake_socket.set_broadcast(true))?;

    let client_handshake_packet =
        trace_err!(bincode::serialize(&HandshakePacket::Client(handshake_packet)))?;

    loop {
        handshake_socket
            .send_to(&client_handshake_packet, (Ipv4Addr::BROADCAST, CONTROL_PORT))
            .await
            .map_err(|_| ConnectionError::NetworkUnreachable)?;

        tokio::select! {
            res = receive_response_loop(&mut handshake_socket) => break res,
            _ = time::sleep(CLIENT_HANDSHAKE_RESEND_INTERVAL) => {
                warn!("Server not found, resending handshake packet");
            }
        }
    }
}

async fn receive_response_loop(
    handshake_socket: &mut UdpSocket
) -> Result<(), ConnectionError> {
    let mut server_response_buffer = [0; MAX_HANDSHAKE_PACKET_SIZE_BYTES];
    loop {
        // this call will receive also the broadcast client packet that must be ignored
        let (packet_size, _) =
            trace_err!(handshake_socket.recv_from(&mut server_response_buffer).await)?;

        let packet = trace_err!(bincode::deserialize(&server_response_buffer[..packet_size]))?;
        if let HandshakePacket::Server(handshake_packet) = packet {
            break match handshake_packet {
                ServerHandshakePacket::ClientUntrusted =>
                    Err(ConnectionError::ClientUntrusted),
                ServerHandshakePacket::IncompatibleVersions =>
                    Err(ConnectionError::IncompatibleVersions)
            };
        }
    }
}

async fn legacy_receive_loop(
    mut legacy_receive_data_receiver: tmpsc::UnboundedReceiver<Vec<u8>>,
    time_sync_sender: tmpsc::UnboundedSender<TimeSync>,
    video_error_report_sender: tmpsc::UnboundedSender<()>,
    codec: CodecType,
    enable_fec: bool,
) -> StrResult {
    let push_nal = buffer_queue::push_nal;
    let mut handler = StreamHandler::new(
        enable_fec,
        codec.into(),
        push_nal,
        time_sync_sender,
        video_error_report_sender,
    );
    let mut idr_request_deadline = None;

    while let Some(data) = legacy_receive_data_receiver.recv().await {
        // Send again IDR packet every 2s in case it is missed
        // (due to dropped burst of packets at the start of the stream or otherwise).
        if !buffer_queue::is_idr_parsed() {
            if let Some(deadline) = idr_request_deadline {
                if deadline < Instant::now() {
                    IDR_REQUEST_NOTIFIER.notify_waiters();
                    idr_request_deadline = None;
                }
            } else {
                idr_request_deadline = Some(Instant::now() + Duration::from_secs(2));
            }
        }
        handler.legacy_receive(data.into());
    }
    info!("legacy_receive_loop finished");
    Ok(())
}

async fn video_receive_loop(
    mut stream_receiver: StreamReceiver<VideoFrameHeaderPacket>,
    legacy_receive_data_sender: tmpsc::UnboundedSender<Vec<u8>>,
) -> StrResult {
    const HEADER_LEN: usize = mem::size_of::<VideoFrameHeader>();
    loop {
        let packet = stream_receiver.recv().await?;
        let mut buffer = vec![0_u8; HEADER_LEN + packet.buffer.len()];
        let header = VideoFrameHeader {
            packet_type: 9, // ALVR_PACKET_TYPE_VIDEO_FRAME
            packet_counter: packet.header.packet_counter,
            tracking_frame_index: packet.header.tracking_frame_index,
            video_frame_index: packet.header.video_frame_index,
            sent_time: packet.header.sent_time,
            frame_byte_size: packet.header.frame_byte_size,
            fec_index: packet.header.fec_index,
            fec_percentage: packet.header.fec_percentage,
        };
        buffer[..HEADER_LEN].copy_from_slice(unsafe {
            &mem::transmute::<_, [u8; HEADER_LEN]>(header)
        });
        buffer[HEADER_LEN..].copy_from_slice(&packet.buffer);
        trace_err!(legacy_receive_data_sender.send(buffer))?;
    }
}

async fn haptics_receive_loop(
    mut stream_receiver: StreamReceiver<Haptics<TrackedDeviceType>>,
    legacy_receive_data_sender: tmpsc::UnboundedSender<Vec<u8>>,
) -> StrResult {
    const HAPTICS_FEEDBACK_LEN: usize = mem::size_of::<HapticsFeedback>();
    loop {
        let packet = stream_receiver.recv().await?;
        let haptics = HapticsFeedback {
            packet_type: 13, // ALVR_PACKET_TYPE_HAPTICS
            start_time: 0,
            amplitude: packet.header.amplitude,
            duration: packet.header.duration.as_secs_f32(),
            frequency: packet.header.frequency,
            hand: if matches!(packet.header.device, TrackedDeviceType::LeftHand) {
                0
            } else {
                1
            },
        };
        let mut buffer = vec![0_u8; HAPTICS_FEEDBACK_LEN];
        buffer.copy_from_slice(unsafe {
            &mem::transmute::<_, [u8; HAPTICS_FEEDBACK_LEN]>(haptics)
        });
        trace_err!(legacy_receive_data_sender.send(buffer))?;
    }
}

async fn control_send_loop(
    mut control_send_data_receiver: tmpsc::Receiver<ClientControlPacket>,
    mut control_sender: ControlSocketSender<ClientControlPacket>,
) -> StrResult {
    while let Some(packet) = control_send_data_receiver.recv().await {
        trace_err!(control_sender.send(&packet).await)?;
    }
    info!("control_send_loop finished");
    Ok(())
}

async fn time_sync_send_loop(
    mut rendered_receiver: tmpsc::UnboundedReceiver<u64>,
    control_sender: tmpsc::Sender<ClientControlPacket>,
) -> StrResult {
    while let Some(frame_index) = rendered_receiver.recv().await {
        latency_controller::rendered2(frame_index);
        if latency_controller::submit(frame_index) {
            // TimeSync here might be an issue but it seems to work fine
            let time_sync = latency_controller::new_time_sync();
            debug!("TimeSync {:?}", time_sync);
            trace_err!(control_sender
                .send(ClientControlPacket::TimeSync(time_sync.into()))
                .await
            )?;
        }
    }
    info!("time_sync_send_loop finished");
    Ok(())
}

async fn time_sync_send_back_loop(
    mut time_sync_receiver: tmpsc::UnboundedReceiver<TimeSync>,
    control_sender: tmpsc::Sender<ClientControlPacket>,
) -> StrResult {
    while let Some(time_sync) = time_sync_receiver.recv().await {
        trace_err!(control_sender
            .send(ClientControlPacket::TimeSync(time_sync.into()))
            .await
        )?;
    }
    info!("time_sync_send_back_loop finished");
    Ok(())
}

async fn video_error_report_send_loop(
    mut video_error_report_trigger_receiver: tmpsc::UnboundedReceiver<()>,
    control_sender: tmpsc::Sender<ClientControlPacket>,
) -> StrResult {
    while let Some(()) = video_error_report_trigger_receiver.recv().await {
        trace_err!(control_sender
            .send(ClientControlPacket::VideoErrorReport)
            .await
        )?;
    }
    info!("video_error_report_send_loop finished");
    Ok(())
}

async fn tracking_loop(
    mut socket_sender: StreamSender<Input>,
) -> StrResult {
    let tracking_interval = Duration::from_secs_f32(TRACKING_INTERVAL);
    let mut deadline = Instant::now();
    let mut frame_index = 0;
    loop {
        frame_index += 1;
        let input = match device::get_tracking(frame_index) {
            Ok(tracking) => {
                let head_orientation = tracking.head_pose_orientation.into();
                let head_position: Vec3 = tracking.head_pose_position.into();
                let head_to_eye_position = head_orientation * Vec3::X * tracking.ipd / 2f32;
                Input {
                    target_timestamp: Default::default(), // TODO predicated_display_time
                    view_configs: vec![
                        ViewConfig {
                            orientation: head_orientation,
                            position: head_position - head_to_eye_position,
                            fov: tracking.l_eye_fov.into(),
                        },
                        ViewConfig {
                            orientation: head_orientation,
                            position: head_position + head_to_eye_position,
                            fov: tracking.r_eye_fov.into(),
                        }
                    ],
                    left_pose_input: HandPoseInput {
                        grip_motion: MotionData {
                            orientation: tracking.l_ctrl.orientation(),
                            position: tracking.l_ctrl.position(),
                            linear_velocity: tracking.l_ctrl.linear_velocity(),
                            angular_velocity: tracking.l_ctrl.angular_velocity(),
                        },
                        hand_tracking_input: None,
                    },
                    right_pose_input: HandPoseInput {
                        grip_motion: MotionData {
                            orientation: tracking.r_ctrl.orientation(),
                            position: tracking.r_ctrl.position(),
                            linear_velocity: tracking.r_ctrl.linear_velocity(),
                            angular_velocity: tracking.r_ctrl.angular_velocity(),
                        },
                        hand_tracking_input: None,
                    },
                    trackers_pose_input: vec![],
                    button_values: Default::default(), // unused for now
                    legacy: LegacyInput {
                        flags: 0,
                        client_time: util::get_timestamp_us(),
                        frame_index,
                        battery: tracking.battery as u64,
                        plugged: tracking.plugged,
                        mounted: 1,
                        controller_flags: [
                            CONTROLLER_FLAG_ENABLE
                                | CONTROLLER_FLAG_OCULUS_QUEST
                                | CONTROLLER_FLAG_LEFT_HAND,
                            CONTROLLER_FLAG_ENABLE
                                | CONTROLLER_FLAG_OCULUS_QUEST,
                        ],
                        buttons: [
                            tracking.l_ctrl.buttons,
                            tracking.r_ctrl.buttons,
                        ],
                        trackpad_position: [
                            Vec2::new(
                                tracking.l_ctrl.trackpad_position_x,
                                tracking.l_ctrl.trackpad_position_y,
                            ),
                            Vec2::new(
                                tracking.r_ctrl.trackpad_position_x,
                                tracking.r_ctrl.trackpad_position_y,
                            ),
                        ],
                        trigger_value: [
                            tracking.l_ctrl.trigger_value,
                            tracking.r_ctrl.trigger_value,
                        ],
                        grip_value: [
                            tracking.l_ctrl.grip_value,
                            tracking.r_ctrl.grip_value,
                        ],
                        controller_battery: [100, 100],
                        ..Default::default()
                    },
                }
            }
            Err(e) => {
                warn!("Tracking data not found: {}", e);
                let head_to_eye_position = Vec3::X * packet::DEFAULT_IPD / 2f32;
                Input {
                    target_timestamp: Default::default(),
                    view_configs: vec![
                        ViewConfig {
                            orientation: Quat::IDENTITY,
                            position: -head_to_eye_position,
                            fov: packet::default_left_eye_fov(),
                        },
                        ViewConfig {
                            orientation: Quat::IDENTITY,
                            position: head_to_eye_position,
                            fov: packet::default_right_eye_fov(),
                        }
                    ],
                    left_pose_input: HandPoseInput {
                        grip_motion: MotionData {
                            orientation: Quat::IDENTITY,
                            position: Vec3::ZERO,
                            linear_velocity: None,
                            angular_velocity: None,
                        },
                        hand_tracking_input: None,
                    },
                    right_pose_input: HandPoseInput {
                        grip_motion: MotionData {
                            orientation: Quat::IDENTITY,
                            position: Vec3::ZERO,
                            linear_velocity: None,
                            angular_velocity: None,
                        },
                        hand_tracking_input: None,
                    },
                    trackers_pose_input: vec![],
                    button_values: Default::default(),
                    legacy: LegacyInput {
                        client_time: util::get_timestamp_us(),
                        frame_index,
                        ..Default::default()
                    },
                }
            }
        };
        latency_controller::tracking(frame_index);
        socket_sender.send_buffer(
            socket_sender.new_buffer(&input, 0)?
        ).await?;
        deadline += tracking_interval;
        time::sleep_until(deadline).await;
    }
}

async fn playspace_sync_loop(
    control_sender: tmpsc::Sender<ClientControlPacket>
) -> StrResult {
    loop {
        // let guardian_data = unsafe { crate::getGuardianData() };
        //
        // if guardian_data.shouldSync {
        //     let perimeter_points = if guardian_data.perimeterPointsCount == 0 {
        //         None
        //     } else {
        //         let perimeter_slice = unsafe {
        //             slice::from_raw_parts(
        //                 guardian_data.perimeterPoints,
        //                 guardian_data.perimeterPointsCount as _,
        //             )
        //         };
        //
        //         let perimeter_points = perimeter_slice
        //             .iter()
        //             .map(|p| Point2::from_slice(&[p[0], p[2]]))
        //             .collect::<Vec<_>>();
        //
        //         Some(perimeter_points)
        //     };
        //     let packet = PlayspaceSyncPacket {
        //         position: Point3::from_slice(&guardian_data.position),
        //         rotation: UnitQuaternion::from_quaternion(Quaternion::new(
        //             guardian_data.rotation[3],
        //             guardian_data.rotation[0],
        //             guardian_data.rotation[1],
        //             guardian_data.rotation[2],
        //         )),
        //         area_width: guardian_data.areaWidth,
        //         area_height: guardian_data.areaHeight,
        //         perimeter_points,
        //     };
        //
        //     control_sender
        //         .lock()
        //         .await
        //         .send(&ClientControlPacket::PlayspaceSync(packet))
        //         .await
        //         .ok();
        // }

        time::sleep(PLAYSPACE_SYNC_INTERVAL).await;
    }
}

async fn keepalive_sender_loop(
    control_sender: tmpsc::Sender<ClientControlPacket>
) -> StrResult {
    loop {
        trace_err!(control_sender.send(ClientControlPacket::KeepAlive).await)?;
        time::sleep(NETWORK_KEEPALIVE_INTERVAL).await;
    }
}

async fn request_idr_loop(
    control_sender: tmpsc::Sender<ClientControlPacket>
) -> StrResult {
    loop {
        IDR_REQUEST_NOTIFIER.notified().await;
        trace_err!(control_sender
            .send(ClientControlPacket::RequestIdr)
            .await
        )?;
    }
}

async fn control_receive_loop(
    mut control_receiver: ControlSocketReceiver<ServerControlPacket>,
    legacy_receive_data_sender: tmpsc::UnboundedSender<Vec<u8>>,
) -> StrResult {
    const TIME_SYNC_LEN: usize = mem::size_of::<TimeSync>();
    loop {
        let control_packet = trace_err!(control_receiver.recv().await)?;
        match control_packet {
            ServerControlPacket::Restarting => {
                notify_event(ConnectionEvent::ServerRestart);
                info!("control_receive_loop finished");
                return Ok(())
            }
            ServerControlPacket::TimeSync(data) => {
                let time_sync = TimeSync {
                    packet_type: 7, // ALVR_PACKET_TYPE_TIME_SYNC
                    mode: data.mode,
                    server_time: data.server_time,
                    client_time: data.client_time,
                    sequence: 0,
                    packets_lost_total: data.packets_lost_total,
                    packets_lost_in_second: data.packets_lost_in_second,
                    average_total_latency: 0,
                    average_send_latency: data.average_send_latency,
                    average_transport_latency: data.average_transport_latency,
                    average_decode_latency: data.average_decode_latency,
                    idle_time: data.idle_time,
                    fec_failure: data.fec_failure,
                    fec_failure_in_second: data.fec_failure_in_second,
                    fec_failure_total: data.fec_failure_total,
                    fps: data.fps,
                    server_total_latency: data.server_total_latency,
                    tracking_recv_frame_index: data.tracking_recv_frame_index,
                };
                let mut buffer = vec![0_u8; TIME_SYNC_LEN];
                buffer.copy_from_slice(unsafe {
                    &mem::transmute::<_, [u8; TIME_SYNC_LEN]>(time_sync)
                });
                trace_err!(legacy_receive_data_sender.send(buffer))?;
            }
            _ => ()
        }
    }
}

fn game_audio_loop<'a>(
    game_audio_receiver: StreamReceiver<()>,
    game_audio_desc: Switch<GameAudioDesc>,
    game_audio_sample_rate: u32,
) -> BoxFuture<'a, StrResult> {
    if let Switch::Enabled(desc) = game_audio_desc {
        return Box::pin(audio::play_audio_loop(
            game_audio_receiver,
            desc.config,
            game_audio_sample_rate,
        ));
    }
    Box::pin(audio::play_audio_loop_nop(game_audio_receiver))
}

fn microphone_loop<'a>(
    microphone_sender: StreamSender<()>,
    microphone_desc: Switch<MicrophoneDesc>,
) -> BoxFuture<'a, StrResult> {
    if let Switch::Enabled(desc) = microphone_desc {
        return Box::pin(audio::record_audio_loop(
            microphone_sender,
            desc.sample_rate,
        ));
    }
    Box::pin(audio::record_audio_loop_nop(microphone_sender))
}

pub fn on_rendered(frame_index: u64) {
    if let Some(sender) = &*MAYBE_RENDERED_SENDER.lock() {
        sender.send(frame_index).ok();
    }
}

#[cfg(test)]
mod tests {
    use crate::device::Device;
    use alvr_sockets::PrivateIdentity;
    use log::LevelFilter;
    use once_cell::sync::Lazy;
    use simple_logger::SimpleLogger;
    use std::{thread, time::Duration};
    use tokio;

    static DEVICE: Lazy<Device> = Lazy::new(|| Device {
        name: "Test Device".into(),
        recommended_eye_width: 1920,
        recommended_eye_height: 1080,
        available_refresh_rates: vec![60.0],
        preferred_refresh_rate: 60.0,
    });

    static IDENTITY: Lazy<PrivateIdentity> = Lazy::new(||
        alvr_sockets::create_identity(Some("test.client.alvr".into())).unwrap()
    );

    #[test]
    #[ignore]
    /// Please specify -- --ignored --nocapture to check the log.
    fn run() {
        SimpleLogger::new()
            .without_timestamps()
            .with_level(LevelFilter::Info)
            .init().unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let future = super::connect(&DEVICE, &IDENTITY).unwrap().unwrap();
        runtime.block_on(future).unwrap();
    }

    #[test]
    fn connect_and_disconnect() {
        SimpleLogger::new()
            .without_timestamps()
            .with_level(LevelFilter::Info)
            .init().unwrap();
        {
            super::connect(&DEVICE, &IDENTITY).unwrap();
            super::connect(&DEVICE, &IDENTITY).unwrap();
            thread::sleep(Duration::from_secs(3));
            super::disconnect();
            super::disconnect();
        }
        {
            super::connect(&DEVICE, &IDENTITY).unwrap();
            thread::sleep(Duration::from_secs(5));
            super::disconnect();
        }
    }
}