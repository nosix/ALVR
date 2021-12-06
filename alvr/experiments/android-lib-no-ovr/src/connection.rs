use crate::{
    audio,
    buffer_queue,
    common::{ConnectionError, ConnectionEvent, ConnectionSettings},
    device::Device,
    legacy_packets::*,
    legacy_stream::StreamHandler,
    latency_controller,
    util,
};
use alvr_common::{
    prelude::*,
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
static MAYBE_LEGACY_SENDER: Lazy<Mutex<Option<tmpsc::UnboundedSender<Vec<u8>>>>> = Lazy::new(|| Mutex::new(None));
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
    private_identity: PrivateIdentity,
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
    identity: PrivateIdentity,
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
        device_name: device.get_name().into(),
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
        recommended_eye_width: device.get_recommended_eye_width(),
        recommended_eye_height: device.get_recommended_eye_height(),
        available_refresh_rates: device.get_available_refresh_rates().to_vec(),
        preferred_refresh_rate: device.get_preferred_refresh_rate(),
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

    let mut stream_socket = tokio::select! {
        res = stream_socket_builder.accept_from_server(server_ip, stream_port) => res?,
        _ = time::sleep(SET_UP_STREAM_TIMEOUT) => {
            return Err(ConnectionError::TimeoutSetUpStream);
        }
    };

    notify_event(ConnectionEvent::Connected {
        settings: ConnectionSettings {
            fps: config_packet.fps,
            codec: settings.video.codec.into(),
            realtime: settings.video.client_request_realtime_decoder,
            dark_mode: settings.extra.client_dark_mode,
            dashboard_url: config_packet.dashboard_url,
        }
    });

    // let is_connected = Arc::new(AtomicBool::new(true));
    // let _stream_guard = StreamCloseGuard {
    //     is_connected: Arc::clone(&is_connected),
    // };

    let tracking_clientside_prediction = match &settings.headset.controllers {
        Switch::Enabled(controllers) => controllers.clientside_prediction,
        Switch::Disabled => false,
    };

    // legacy_send_data_sender is used by the sync context.
    let (legacy_send_data_sender, legacy_send_data_receiver) = tmpsc::unbounded_channel();
    *MAYBE_LEGACY_SENDER.lock() = Some(legacy_send_data_sender);

    let legacy_send_loop = legacy_send_loop(
        legacy_send_data_receiver,
        stream_socket.request_stream::<_, LEGACY>().await?,
    );

    let legacy_receive_loop = legacy_receive_loop(
        stream_socket.subscribe_to_stream::<(), LEGACY>().await?,
        settings.video.codec,
        settings.connection.enable_fec,
    );

    let (control_send_data_sender, control_send_data_receiver) = tmpsc::channel(CHANNEL_BUFFER_SIZE);

    let control_send_loop = control_send_loop(
        control_send_data_receiver,
        control_sender,
    );

    let tracking_loop = tracking_loop();
    let playspace_sync_loop = playspace_sync_loop(control_send_data_sender.clone());
    let keepalive_sender_loop = keepalive_sender_loop(control_send_data_sender.clone());

    let control_loop = control_loop(
        control_receiver,
        control_send_data_sender.clone(),
    );

    let game_audio_loop = game_audio_loop(
        stream_socket.subscribe_to_stream().await?,
        settings.audio.game_audio,
        config_packet.game_audio_sample_rate,
    );
    let microphone_loop = microphone_loop(
        stream_socket.request_stream().await?,
        settings.audio.microphone,
    );

    // Run many tasks concurrently. Threading is managed by the runtime, for best performance.
    (tokio::select! {
        res = spawn_cancelable(stream_socket.receive_loop()) => {
            if let Err(e) = res {
                return Err(ConnectionError::ServerDisconnected {
                    cause: format!("{}\n{}", e, trace_str!())
                });
            }
            Ok(())
        },
        res = spawn_cancelable(game_audio_loop) => trace_err!(res),
        res = spawn_cancelable(microphone_loop) => trace_err!(res),
        res = spawn_cancelable(tracking_loop) => trace_err!(res),
        res = spawn_cancelable(playspace_sync_loop) => trace_err!(res),
        res = spawn_cancelable(legacy_send_loop) => trace_err!(res),
        res = spawn_cancelable(legacy_receive_loop) => trace_err!(res),
        res = buffer_queue::buffer_coordination_loop() => trace_err!(res)?,

        // keep these loops on the current task
        res = keepalive_sender_loop => trace_err!(res),
        res = control_send_loop => trace_err!(res),
        res = control_loop => trace_err!(res),
    })?;

    Ok(())
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

async fn legacy_send_loop(
    mut legacy_send_data_receiver: tmpsc::UnboundedReceiver<Vec<u8>>,
    mut socket_sender: StreamSender<(), LEGACY>,
) -> StrResult {
    while let Some(data) = legacy_send_data_receiver.recv().await {
        let mut buffer = socket_sender.new_buffer(&(), data.len())?;
        buffer.get_mut().extend(data);
        socket_sender.send_buffer(buffer).await.ok();
    }
    Ok(())
}

async fn legacy_receive_loop(
    mut socket_receiver: StreamReceiver<(), LEGACY>,
    codec: CodecType,
    enable_fec: bool,
) -> StrResult {
    let push_nal = buffer_queue::push_nal;
    let mut handler = StreamHandler::new(enable_fec, codec.into(), push_nal, legacy_send);
    let mut idr_request_deadline = None;

    while let packet = socket_receiver.recv().await? {
        let data = packet.buffer;

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

        // crate::IS_CONNECTED.store(true, Ordering::Relaxed);

        handler.legacy_receive(data.freeze());
    }

    // crate::IS_CONNECTED.store(false, Ordering::Relaxed);

    Ok(())
}

async fn control_send_loop(
    mut control_send_data_receiver: tmpsc::Receiver<ClientControlPacket>,
    mut control_sender: ControlSocketSender<ClientControlPacket>,
) -> StrResult {
    while let Some(packet) = control_send_data_receiver.recv().await {
        trace_err!(control_sender.send(&packet).await)?;
    }
    Ok(())
}

async fn tracking_loop() -> StrResult {
    let tracking_interval = Duration::from_secs_f32(TRACKING_INTERVAL);
    let mut deadline = Instant::now();
    let mut frame_index = 0;
    loop {
        // unsafe { crate::onTrackingNative(tracking_clientside_prediction) };
        frame_index += 1;
        let tracking_info = TrackingInfo {
            client_time: util::get_timestamp_us(),
            frame_index,
            // TODO predicated_display_time
            ipd: 0.068606f32,
            // TODO eye_fov
            // TODO battery
            // TODO plugged
            // TODO head_pose_orientation
            // TODO head_pose_position
            // TODO controller
            ..Default::default()
        };
        latency_controller::INSTANCE.lock().tracking(frame_index);
        legacy_send(tracking_info.into());

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
        info!("send PlayspaceSync");

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

async fn control_loop(
    mut control_receiver: ControlSocketReceiver<ServerControlPacket>,
    control_sender: tmpsc::Sender<ClientControlPacket>,
) -> StrResult {
    loop {
        tokio::select! {
            _ = IDR_REQUEST_NOTIFIER.notified() => {
                trace_err!(control_sender.send(ClientControlPacket::RequestIdr).await)?;
            }
            control_packet = control_receiver.recv() => match trace_err!(control_packet)? {
                ServerControlPacket::Restarting => {
                    notify_event(ConnectionEvent::ServerRestart);
                    return Ok(())
                }
                _ => ()
            }
        }
    }
}

fn game_audio_loop<'a>(
    game_audio_receiver: StreamReceiver<(), AUDIO>,
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
    microphone_sender: StreamSender<(), AUDIO>,
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

fn legacy_send(message: Vec<u8>) {
    if let Some(sender) = &*MAYBE_LEGACY_SENDER.lock() {
        sender.send(message).ok();
    }
}

#[cfg(test)]
mod tests {
    use crate::device::Device;
    use alvr_sockets::PrivateIdentity;
    use once_cell::sync::Lazy;
    use simple_logger::SimpleLogger;
    use std::{thread, time::Duration};
    use log::LevelFilter;
    use tokio;

    static DEVICE: Lazy<Device> = Lazy::new(|| Device::new("Test Device"));

    fn clone_identity(identity: &PrivateIdentity) -> PrivateIdentity {
        PrivateIdentity {
            hostname: identity.hostname.clone(),
            certificate_pem: identity.certificate_pem.clone(),
            key_pem: identity.key_pem.clone(),
        }
    }

    #[test]
    #[ignore]
    /// Please specify -- --ignored --nocapture to check the log.
    fn run() {
        SimpleLogger::new().with_level(LevelFilter::Info).init().unwrap();
        let identity =
            alvr_sockets::create_identity(Some("test.client.alvr".into())).unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let future = super::connect(&DEVICE, identity).unwrap().unwrap();
        runtime.block_on(future).unwrap();
    }

    #[test]
    fn connect_and_disconnect() {
        SimpleLogger::new().with_level(LevelFilter::Info).init().unwrap();
        let identity =
            alvr_sockets::create_identity(Some("test.client.alvr".into())).unwrap();
        {
            super::connect(&DEVICE, clone_identity(&identity)).unwrap();
            super::connect(&DEVICE, clone_identity(&identity)).unwrap();
            thread::sleep(Duration::from_secs(3));
            super::disconnect();
            super::disconnect();
        }
        {
            super::connect(&DEVICE, clone_identity(&identity)).unwrap();
            thread::sleep(Duration::from_secs(5));
            super::disconnect();
        }
    }
}